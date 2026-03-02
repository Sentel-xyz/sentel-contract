use crate::{contexts::CancelTransaction, errors::CustomError, TransactionCancelledEvent};
use anchor_lang::prelude::*;

pub fn cancel_transaction(
    ctx: Context<CancelTransaction>,
    _creator: Pubkey,
    _vault_id: u64,
    transaction_nonce: u64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let transaction = &mut ctx.accounts.transaction;
    let signer = &ctx.accounts.signer;

    // Only vault owners can vote to cancel
    require!(
        vault.owners.contains(&signer.key()),
        CustomError::UnauthorizedProposer
    );

    // Can't cancel an already executed transaction
    require!(
        !transaction.executed,
        CustomError::TransactionAlreadyExecuted
    );

    // Prevent double-voting
    require!(
        !transaction.cancellations.contains(&signer.key()),
        CustomError::AlreadyCancelledVote
    );

    transaction.cancellations.push(signer.key());

    // Reached threshold -> cancel the proposal
    if transaction.cancellations.len() >= vault.threshold as usize {
        vault
            .pending_transactions
            .retain(|&id| id != transaction_nonce);

        emit!(TransactionCancelledEvent {
            transaction_id: transaction_nonce,
            cancelled_by: signer.key(),
            vault: vault.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
    }

    Ok(())
}
