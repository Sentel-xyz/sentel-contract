use crate::contexts::ApproveRetrieveTransaction;
use crate::errors::CustomError;
use anchor_lang::prelude::*;

pub fn approve_retrieve_transaction(
    ctx: Context<ApproveRetrieveTransaction>,
    _vault_id: u64,
    _transaction_nonce: u64,
) -> Result<()> {
    let balanced_vault = &ctx.accounts.balanced_vault;
    let retrieve_transaction = &mut ctx.accounts.retrieve_transaction;
    let approver = &ctx.accounts.approver;

    // Verify approver is an owner
    require!(
        balanced_vault.owners.contains(&approver.key()),
        CustomError::Unauthorized
    );

    // Check if already executed
    require!(
        !retrieve_transaction.executed,
        CustomError::TransactionAlreadyExecuted
    );

    // Check expiration
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp < retrieve_transaction.expires_at,
        CustomError::TransactionExpired
    );

    // Check if already approved
    require!(
        !retrieve_transaction.approvals.contains(&approver.key()),
        CustomError::AlreadyApproved
    );

    // Check if proposal is still active (not cancelled by threshold vote)
    require!(
        balanced_vault
            .pending_transactions
            .contains(&retrieve_transaction.id),
        CustomError::ProposalCancelled
    );

    // Add approval
    retrieve_transaction.approvals.push(approver.key());

    Ok(())
}
