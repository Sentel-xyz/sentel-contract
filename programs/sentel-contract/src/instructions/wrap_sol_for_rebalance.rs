use crate::contexts::WrapSolForRebalance;
use crate::errors::CustomError;
use anchor_lang::prelude::*;

/// Wraps native SOL from the balanced vault PDA into its WSOL token account,
/// taking the protocol fee. This must be called in a SEPARATE transaction before
/// `rebalance_vault`, because Solana disallows CPIs after direct lamport
/// manipulation on token-program-owned accounts within the same instruction.
pub fn wrap_sol_for_rebalance(ctx: Context<WrapSolForRebalance>, _vault_id: u64) -> Result<()> {
    let balanced_vault = &ctx.accounts.balanced_vault;

    require!(
        balanced_vault.is_active,
        CustomError::BalancedVaultNotActive
    );

    let rebalancer_key = ctx.accounts.rebalancer.key();
    require!(
        balanced_vault.owners.contains(&rebalancer_key),
        CustomError::Unauthorized
    );

    let vault_info = balanced_vault.to_account_info();
    let vault_lamports = vault_info.lamports();

    // Calculate rent-exempt reserve for BalancedVaultState, plus a 0.1 SOL safety
    // margin and a typical transaction fee buffer (~5000 lamports).
    let rent = Rent::get()?;
    let rent_exempt_reserve = rent.minimum_balance(vault_info.data_len());
    const SAFETY_MARGIN: u64 = 100_000_000; // 0.1 SOL
    const TX_FEE_BUFFER: u64 = 5_000; // ~2 signatures × ~2500 lamports each
    let reserve = rent_exempt_reserve
        .saturating_add(SAFETY_MARGIN)
        .saturating_add(TX_FEE_BUFFER);

    // Available SOL = total lamports minus the reserve
    let available_sol = vault_lamports.saturating_sub(reserve);

    // Protocol fee: 0.05% of available SOL, clamped between MIN_FEE_LAMPORTS and MAX_FEE_LAMPORTS.
    let fee_bps = crate::PROTOCOL_FEE_BASIS_POINTS; // 5 bps = 0.05%
    let raw_fee = available_sol
        .saturating_mul(fee_bps)
        .checked_div(10_000)
        .unwrap_or(0);
    let fee_amount = raw_fee
        .max(crate::MIN_FEE_LAMPORTS)
        .min(crate::MAX_FEE_LAMPORTS);

    require!(available_sol > fee_amount, CustomError::InsufficientFunds);

    let wrap_amount = available_sol
        .checked_sub(fee_amount)
        .ok_or(CustomError::InsufficientFunds)?;

    let wsol_account_info = ctx.accounts.vault_wsol_account.to_account_info();
    let fee_recipient_info = ctx.accounts.fee_recipient.to_account_info();

    // Direct lamport manipulation: move SOL from vault PDA -> WSOL ATA + fee recipient.
    // The vault PDA is owned by this program so we can modify its lamports.
    // We can add lamports to the WSOL ATA (owned by Token program) because
    // only REMOVING lamports from another program's account is restricted.
    // NOTE: No CPI is done after this  sync_native must be called in the next transaction.
    **vault_info.try_borrow_mut_lamports()? = vault_info
        .lamports()
        .checked_sub(available_sol)
        .ok_or(CustomError::InsufficientFunds)?;
    **wsol_account_info.try_borrow_mut_lamports()? = wsol_account_info
        .lamports()
        .checked_add(wrap_amount)
        .ok_or(CustomError::InsufficientFunds)?;
    **fee_recipient_info.try_borrow_mut_lamports()? = fee_recipient_info
        .lamports()
        .checked_add(fee_amount)
        .ok_or(CustomError::InsufficientFunds)?;

    Ok(())
}
