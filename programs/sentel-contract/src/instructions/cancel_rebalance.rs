use crate::{contexts::CancelRebalance, errors::CustomError};
use anchor_lang::prelude::*;

pub fn cancel_rebalance(
    ctx: Context<CancelRebalance>,
    _vault_id: u64,
    proposal_nonce: u64,
) -> Result<()> {
    let balanced_vault = &mut ctx.accounts.balanced_vault;
    let rebalance_proposal = &mut ctx.accounts.rebalance_proposal;
    let signer = &ctx.accounts.signer;

    // Only vault owners can vote to cancel
    require!(
        balanced_vault.owners.contains(&signer.key()),
        CustomError::Unauthorized
    );

    // Can't cancel an already executed proposal
    require!(
        !rebalance_proposal.executed,
        CustomError::TransactionAlreadyExecuted
    );

    // Prevent double-voting
    require!(
        !rebalance_proposal.cancellations.contains(&signer.key()),
        CustomError::AlreadyCancelledVote
    );

    rebalance_proposal.cancellations.push(signer.key());

    // Reached threshold -> cancel the proposal (remove from pending)
    if rebalance_proposal.cancellations.len() >= balanced_vault.threshold as usize {
        balanced_vault
            .pending_transactions
            .retain(|&id| id != proposal_nonce);
    }

    Ok(())
}
