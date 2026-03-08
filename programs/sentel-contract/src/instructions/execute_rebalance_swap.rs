use crate::contexts::ExecuteRebalanceSwap;
use crate::errors::CustomError;
use crate::instructions::jupiter_account_meta;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token::{sync_native, SyncNative};
use std::str::FromStr;

/// Execute a single Jupiter swap from an approved rebalance proposal, without closing the PDA.
/// Call once per allocation. After all swaps are done, call `finalize_rebalance` to close the PDA.
///
/// `swap_index`  zero-based index of the swap being executed (must equal `swaps_executed`).
pub fn execute_rebalance_swap<'info>(
    ctx: Context<'_, '_, '_, 'info, ExecuteRebalanceSwap<'info>>,
    vault_id: u64,
    proposal_nonce: u64,
    swap_index: u32,
    total_swaps: u32,
    jupiter_swap_data: Vec<u8>,
    swap_account_count: u32,
) -> Result<()> {
    // ---- validation --------------------------------------------------------
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

        // Enforce sequential execution: swap_index must match swaps_executed counter.
        require!(
            swap_index == rebalance_proposal.swaps_executed,
            CustomError::InvalidAmount // re-use generic error; no "wrong swap index" variant yet
        );

        require!(total_swaps > 0, CustomError::InvalidAmount);
    }

    let jupiter_program_id = Pubkey::from_str(crate::JUPITER_V6_PROGRAM_ID)
        .map_err(|_| CustomError::InvalidFeeRecipient)?;

    require!(
        ctx.accounts.jupiter_program.key() == jupiter_program_id,
        CustomError::InvalidFeeRecipient
    );

    // sync_native only on the FIRST swap (once is enough per rebalance)
    if swap_index == 0 {
        let wsol_lamports = ctx.accounts.vault_wsol_account.to_account_info().lamports();
        let wsol_rent = Rent::get()?.minimum_balance(165);
        let effective_wsol = wsol_lamports.saturating_sub(wsol_rent);
        require!(
            effective_wsol > 0,
            CustomError::InsufficientWsolForRebalance
        );

        let sync_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            SyncNative {
                account: ctx.accounts.vault_wsol_account.to_account_info(),
            },
        );
        sync_native(sync_ctx)?;
    }

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

    let balanced_vault_key = ctx.accounts.balanced_vault.key();
    let remaining_accounts = ctx.remaining_accounts;
    let count = swap_account_count as usize;

    require!(
        remaining_accounts.len() == count,
        CustomError::InvalidAmount
    );

    let swap_accounts: Vec<_> = remaining_accounts
        .iter()
        .map(|acc| jupiter_account_meta(acc, &balanced_vault_key))
        .collect();

    let jupiter_ix = Instruction {
        program_id: jupiter_program_id,
        accounts: swap_accounts,
        data: jupiter_swap_data,
    };

    invoke_signed(&jupiter_ix, remaining_accounts, signer_seeds)?;

    // Increment the counter. If all swaps are done, mark executed so finalize_rebalance
    // can verify everything ran (and so a stale proposal can't be re-executed).
    ctx.accounts.rebalance_proposal.swaps_executed += 1;
    if ctx.accounts.rebalance_proposal.swaps_executed >= total_swaps {
        ctx.accounts.rebalance_proposal.executed = true;
    }

    Ok(())
}
