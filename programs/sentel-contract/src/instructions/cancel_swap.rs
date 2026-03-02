use crate::{contexts::CancelSwap, errors::CustomError};
use anchor_lang::prelude::*;

pub fn cancel_swap(
    ctx: Context<CancelSwap>,
    _creator: Pubkey,
    _vault_id: u64,
    _swap_nonce: u64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let swap_transaction = &mut ctx.accounts.swap_transaction;
    let signer = &ctx.accounts.signer;

    require!(
        vault.owners.contains(&signer.key()),
        CustomError::UnauthorizedProposer
    );

    require!(!swap_transaction.executed, CustomError::SwapAlreadyExecuted);

    require!(
        !swap_transaction.cancellations.contains(&signer.key()),
        CustomError::AlreadyCancelledVote
    );

    swap_transaction.cancellations.push(signer.key());

    // Reached threshold -> cancel the proposal
    if swap_transaction.cancellations.len() >= vault.threshold as usize {
        vault
            .pending_transactions
            .retain(|&id| id != swap_transaction.id);
    }

    Ok(())
}
