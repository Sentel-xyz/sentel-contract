use anchor_lang::prelude::*;

use crate::{contexts::ApproveTransaction, errors::CustomError};

pub fn approve_transaction(
    ctx: Context<ApproveTransaction>,
    _creator: Pubkey,
    _vault_id: u64,
    _transaction_nonce: u64,
) -> Result<()> {
    let vault = &ctx.accounts.vault;
    let transaction = &mut ctx.accounts.transaction;
    let signer = &ctx.accounts.signer;

    // Signer needs to be an owner
    require!(
        vault.owners.contains(&signer.key()),
        CustomError::UnauthorizedProposer
    );

    // Check if signer already approved this transaction
    require!(
        !transaction.approvals.contains(&signer.key()),
        CustomError::AlreadyApproved
    );

    // Check if transaction is not already executed
    require!(
        !transaction.executed,
        CustomError::TransactionAlreadyExecuted
    );

    // Check if proposal is still active (not cancelled by threshold vote)
    require!(
        vault.pending_transactions.contains(&transaction.id),
        CustomError::ProposalCancelled
    );

    // Check if transaction has not expired
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp <= transaction.expires_at,
        CustomError::TransactionExpired
    );

    transaction.approvals.push(signer.key());

    Ok(())
}
