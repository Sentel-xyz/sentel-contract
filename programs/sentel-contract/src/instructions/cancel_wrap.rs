use crate::{contexts::CancelWrap, errors::CustomError};
use anchor_lang::prelude::*;

pub fn cancel_wrap(
    ctx: Context<CancelWrap>,
    _creator: Pubkey,
    _vault_id: u64,
    _wrap_nonce: u64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let wrap_transaction = &mut ctx.accounts.wrap_transaction;
    let signer = &ctx.accounts.signer;

    require!(
        vault.owners.contains(&signer.key()),
        CustomError::UnauthorizedProposer
    );

    require!(
        !wrap_transaction.executed,
        CustomError::TransactionAlreadyExecuted
    );

    require!(
        !wrap_transaction.cancellations.contains(&signer.key()),
        CustomError::AlreadyCancelledVote
    );

    wrap_transaction.cancellations.push(signer.key());

    // Reached threshold -> cancel the proposal
    if wrap_transaction.cancellations.len() >= vault.threshold as usize {
        vault
            .pending_transactions
            .retain(|&id| id != wrap_transaction.id);
    }

    Ok(())
}
