use crate::{contexts::ProposeSolSwap, errors::CustomError};
use anchor_lang::prelude::*;
use std::str::FromStr;

/// Proposes a native-SOL-to-token swap for a standard multisig vault.
///
/// The SOL is wrapped to WSOL atomically at propose-time:
///   vault lamports -> vault WSOL ATA (sol_amount)
///
/// No fee is charged here. The protocol fee is collected at execution time
/// in `execute_swap`, after the Jupiter swap succeeds.
/// After this instruction the swap PDA's input_mint is WSOL, so
/// `approve_swap` / `execute_swap` work identically to any other swap.
/// The WSOL ATA lamports are increased here; `execute_swap` will call
/// sync_native before swapping.
pub fn propose_sol_swap(
    ctx: Context<ProposeSolSwap>,
    output_mint: Pubkey,
    sol_amount: u64,
    minimum_output_amount: u64,
    _vault_id: u64,
    _creator: Pubkey,
) -> Result<()> {
    require!(sol_amount > 0, CustomError::InvalidAmount);
    require!(minimum_output_amount > 0, CustomError::InvalidAmount);

    let vault = &mut ctx.accounts.vault;
    let swap_transaction = &mut ctx.accounts.swap_transaction;

    require!(
        vault.owners.contains(&ctx.accounts.proposer.key()),
        CustomError::UnauthorizedProposer
    );

    require!(
        vault.pending_transactions.len() < 50,
        CustomError::TooManyPendingTransactions
    );

    let wsol_mint = Pubkey::from_str(crate::WSOL_MINT).unwrap();

    // Verify the vault PDA has enough lamports above rent-exempt minimum for the wrap.
    let vault_info = vault.to_account_info();
    let vault_lamports = vault_info.lamports();
    let rent_exempt = Rent::get()?.minimum_balance(vault_info.data_len());
    let spendable = vault_lamports.saturating_sub(rent_exempt);
    require!(spendable >= sol_amount, CustomError::InsufficientFunds);

    // ── Atomic lamport wrap ──────────────────────────────────────────────────
    // NOTE: Anchor has already executed the `init` CPI (system_program::create_account
    // for swap_transaction) before this handler runs. No further CPI is issued below,
    // so the Solana runtime constraint "no CPI after lamport mutation on token accounts"
    // is satisfied.

    let wsol_info = ctx.accounts.vault_wsol_account.to_account_info();

    **vault_info.try_borrow_mut_lamports()? = vault_lamports
        .checked_sub(sol_amount)
        .ok_or(CustomError::InsufficientFunds)?;

    **wsol_info.try_borrow_mut_lamports()? = wsol_info
        .lamports()
        .checked_add(sol_amount)
        .ok_or(CustomError::InsufficientFunds)?;

    // ── Initialise the swap PDA ──────────────────────────────────────────────
    // input_mint = WSOL so that execute_swap can use the standard path with
    // vault_input_token_account = vault's WSOL ATA.
    let clock = Clock::get()?;

    swap_transaction.id = vault.nonce;
    swap_transaction.proposer = ctx.accounts.proposer.key();
    swap_transaction.input_mint = wsol_mint;
    swap_transaction.output_mint = output_mint;
    swap_transaction.input_amount = sol_amount;
    swap_transaction.minimum_output_amount = minimum_output_amount;
    swap_transaction.approvals = Vec::new();
    swap_transaction.cancellations = Vec::new();
    swap_transaction.executed = false;
    swap_transaction.created_at = clock.unix_timestamp;
    swap_transaction.expires_at = clock
        .unix_timestamp
        .checked_add(crate::TRANSACTION_EXPIRY_SECONDS)
        .ok_or(CustomError::InvalidAmount)?;

    vault.pending_transactions.push(swap_transaction.id);
    vault.nonce = vault
        .nonce
        .checked_add(1)
        .ok_or(CustomError::InvalidAmount)?;

    Ok(())
}
