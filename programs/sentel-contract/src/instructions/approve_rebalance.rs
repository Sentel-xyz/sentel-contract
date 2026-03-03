use crate::contexts::ApproveRebalance;
use crate::errors::CustomError;
use anchor_lang::prelude::*;

pub fn approve_rebalance(
    ctx: Context<ApproveRebalance>,
    _vault_id: u64,
    _proposal_nonce: u64,
) -> Result<()> {
    let balanced_vault = &ctx.accounts.balanced_vault;
    let rebalance_proposal = &mut ctx.accounts.rebalance_proposal;
    let approver = &ctx.accounts.approver;

    // Verify approver is an owner
    require!(
        balanced_vault.owners.contains(&approver.key()),
        CustomError::Unauthorized
    );

    // Check if already executed
    require!(
        !rebalance_proposal.executed,
        CustomError::TransactionAlreadyExecuted
    );

    // Check expiration
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp < rebalance_proposal.expires_at,
        CustomError::TransactionExpired
    );

    // Check if already approved
    require!(
        !rebalance_proposal.approvals.contains(&approver.key()),
        CustomError::AlreadyApproved
    );

    // Check if proposal is still active (not cancelled by threshold vote)
    require!(
        balanced_vault
            .pending_transactions
            .contains(&rebalance_proposal.id),
        CustomError::ProposalCancelled
    );

    // Add approval
    rebalance_proposal.approvals.push(approver.key());

    Ok(())
}
