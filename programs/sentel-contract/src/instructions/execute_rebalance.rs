use crate::contexts::ExecuteRebalance;
use crate::errors::CustomError;
use crate::instructions::jupiter_account_meta;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token::{sync_native, SyncNative};
use std::str::FromStr;

/// Executes an approved rebalance proposal. The proposer (or any owner) calls this
/// after enough co-owners have approved via `approve_rebalance`.
/// Works identically to `rebalance_vault` but checks the multisig approval gate first
/// and closes the proposal PDA on success (rent returned to executor).
pub fn execute_rebalance<'info>(
    ctx: Context<'_, '_, '_, 'info, ExecuteRebalance<'info>>,
    vault_id: u64,
    proposal_nonce: u64,
    jupiter_swap_data: Vec<Vec<u8>>,
    swap_account_counts: Vec<u32>,
) -> Result<()> {
    // ---- validation (immutable borrows) ------------------------------------
    {
        let balanced_vault = &ctx.accounts.balanced_vault;
        let rebalance_proposal = &ctx.accounts.rebalance_proposal;

        require!(
            balanced_vault.is_active,
            CustomError::BalancedVaultNotActive
        );

        require!(
            balanced_vault.owners.contains(&ctx.accounts.executor.key()),
            CustomError::Unauthorized
        );

        require!(
            rebalance_proposal.id == proposal_nonce,
            CustomError::RebalanceProposalNotFound
        );

        require!(
            !rebalance_proposal.executed,
            CustomError::TransactionAlreadyExecuted
        );

        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp < rebalance_proposal.expires_at,
            CustomError::TransactionExpired
        );

        require!(
            balanced_vault
                .pending_transactions
                .contains(&rebalance_proposal.id),
            CustomError::ProposalCancelled
        );

        require!(
            rebalance_proposal.approvals.len() >= balanced_vault.threshold as usize,
            CustomError::InsufficientApprovalsForRebalance
        );
    }

    let jupiter_program_id = Pubkey::from_str(crate::JUPITER_V6_PROGRAM_ID)
        .map_err(|_| CustomError::InvalidFeeRecipient)?;

    require!(
        ctx.accounts.jupiter_program.key() == jupiter_program_id,
        CustomError::InvalidFeeRecipient
    );

    require!(
        !jupiter_swap_data.is_empty(),
        CustomError::InsufficientWsolForRebalance
    );

    require!(
        swap_account_counts.len() == jupiter_swap_data.len(),
        CustomError::InvalidAmount
    );

    // Build PDA signer seeds
    let vault_id_bytes = vault_id.to_le_bytes();
    let creator_key = ctx.accounts.balanced_vault.creator;
    let bump = ctx.accounts.balanced_vault.bump;
    let seeds = &[
        b"balanced_vault".as_ref(),
        creator_key.as_ref(),
        vault_id_bytes.as_ref(),
        &[bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // sync_native: update WSOL token account amount to match lamport balance
    let sync_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        SyncNative {
            account: ctx.accounts.vault_wsol_account.to_account_info(),
        },
    );
    sync_native(sync_ctx)?;

    // Verify WSOL balance is sufficient
    let wsol_lamports = ctx.accounts.vault_wsol_account.to_account_info().lamports();
    let wsol_rent = Rent::get()?.minimum_balance(165);
    let effective_wsol = wsol_lamports.saturating_sub(wsol_rent);
    require!(
        effective_wsol > 0,
        CustomError::InsufficientWsolForRebalance
    );

    // Execute Jupiter swaps
    let remaining_accounts = ctx.remaining_accounts;
    let mut account_offset: usize = 0;
    let balanced_vault_key = ctx.accounts.balanced_vault.key();

    for (i, swap_data) in jupiter_swap_data.iter().enumerate() {
        let account_count = swap_account_counts[i] as usize;
        let swap_account_infos =
            &remaining_accounts[account_offset..account_offset + account_count];

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

    // Mark executed and remove from pending
    ctx.accounts.rebalance_proposal.executed = true;
    ctx.accounts
        .balanced_vault
        .pending_transactions
        .retain(|&id| id != proposal_nonce);

    Ok(())
}
