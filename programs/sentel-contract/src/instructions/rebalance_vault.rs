use crate::contexts::RebalanceVault;
use crate::errors::CustomError;
use crate::instructions::jupiter_account_meta;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token::{sync_native, SyncNative};
use std::str::FromStr;

/// Executes Jupiter swaps to rebalance the vault's WSOL into the target token allocations.
/// Call `wrap_sol_for_rebalance` in a SEPARATE prior transaction to move SOL into the WSOL ATA.
/// This instruction calls sync_native (safe as the first CPI, no prior lamport manip in this tx),
/// then executes the Jupiter swaps.
pub fn rebalance_vault<'info>(
    ctx: Context<'_, '_, '_, 'info, RebalanceVault<'info>>,
    vault_id: u64,
    jupiter_swap_data: Vec<Vec<u8>>,
    swap_account_counts: Vec<u32>,
) -> Result<()> {
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

    let jupiter_program_id = Pubkey::from_str(crate::JUPITER_V6_PROGRAM_ID)
        .map_err(|_| CustomError::InvalidFeeRecipient)?;

    require!(
        ctx.accounts.jupiter_program.key() == jupiter_program_id,
        CustomError::InvalidFeeRecipient
    );

    let vault_id_bytes = vault_id.to_le_bytes();
    let creator_key = balanced_vault.creator;
    let seeds = &[
        b"balanced_vault".as_ref(),
        creator_key.as_ref(),
        vault_id_bytes.as_ref(),
        &[balanced_vault.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // Require at least one swap instruction
    require!(
        !jupiter_swap_data.is_empty(),
        CustomError::InsufficientWsolForRebalance
    );

    // Step 1: sync_native  update the WSOL token account's `amount` field to match
    // its current lamport balance (populated by wrap_sol_for_rebalance in the prior tx).
    // This is safe as the FIRST CPI in this instruction (no prior lamport manip).
    let sync_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        SyncNative {
            account: ctx.accounts.vault_wsol_account.to_account_info(),
        },
    );
    sync_native(sync_ctx)?;

    // Step 2: Verify WSOL balance is sufficient.
    let wsol_lamports = ctx.accounts.vault_wsol_account.to_account_info().lamports();
    let wsol_rent = Rent::get()?.minimum_balance(165); // SPL token account size
    let effective_wsol = wsol_lamports.saturating_sub(wsol_rent);

    require!(
        effective_wsol > 0,
        CustomError::InsufficientWsolForRebalance
    );

    // Step 3: Execute Jupiter swaps via CPI

    // L-3 fix: ensure swap_account_counts is aligned with swap data to prevent
    // out-of-bounds slicing and silent account misallocation.
    require!(
        swap_account_counts.len() == jupiter_swap_data.len(),
        CustomError::InvalidAmount
    );

    let remaining_accounts = ctx.remaining_accounts;
    let mut account_offset: usize = 0;

    for (i, swap_data) in jupiter_swap_data.iter().enumerate() {
        let account_count = swap_account_counts[i] as usize;

        let swap_account_infos =
            &remaining_accounts[account_offset..account_offset + account_count];

        let balanced_vault_key = ctx.accounts.balanced_vault.key();
        let swap_accounts: Vec<_> = swap_account_infos
            .iter()
            .map(|acc| jupiter_account_meta(acc, &balanced_vault_key))
            .collect();

        let jupiter_ix = Instruction {
            program_id: jupiter_program_id,
            accounts: swap_accounts,
            data: swap_data.clone(),
        };

        invoke_signed(&jupiter_ix, swap_account_infos, signer_seeds)?;

        account_offset += account_count;
    }

    Ok(())
}
