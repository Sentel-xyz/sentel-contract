use crate::{contexts::CancelRetrieveTransaction, errors::CustomError};
use anchor_lang::prelude::*;

pub fn cancel_retrieve_transaction(
    ctx: Context<CancelRetrieveTransaction>,
    _vault_id: u64,
    transaction_nonce: u64,
) -> Result<()> {
    let balanced_vault = &mut ctx.accounts.balanced_vault;
    let retrieve_transaction = &mut ctx.accounts.retrieve_transaction;
    let signer = &ctx.accounts.signer;

    // Only vault owners can vote to cancel
    require!(
        balanced_vault.owners.contains(&signer.key()),
        CustomError::Unauthorized
    );

    // Can't cancel an already executed transaction
    require!(
        !retrieve_transaction.executed,
        CustomError::TransactionAlreadyExecuted
    );

    // Prevent double-voting
    require!(
        !retrieve_transaction.cancellations.contains(&signer.key()),
        CustomError::AlreadyCancelledVote
    );

    retrieve_transaction.cancellations.push(signer.key());

    // Reached threshold -> cancel the proposal
    if retrieve_transaction.cancellations.len() >= balanced_vault.threshold as usize {
        balanced_vault
            .pending_transactions
            .retain(|&id| id != transaction_nonce);
    }

    Ok(())
}
