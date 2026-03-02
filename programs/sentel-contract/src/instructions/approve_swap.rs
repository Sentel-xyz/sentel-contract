use crate::{contexts::ApproveSwap, errors::CustomError};
use anchor_lang::prelude::*;

pub fn approve_swap(
    ctx: Context<ApproveSwap>,
    _creator: Pubkey,
    _vault_id: u64,
    _swap_nonce: u64,
) -> Result<()> {
    let vault = &ctx.accounts.vault;
    let swap_transaction = &mut ctx.accounts.swap_transaction;
    let signer = &ctx.accounts.signer;

    require!(
        vault.owners.contains(&signer.key()),
        CustomError::UnauthorizedProposer
    );

    require!(!swap_transaction.executed, CustomError::SwapAlreadyExecuted);

    require!(
        !swap_transaction.approvals.contains(&signer.key()),
        CustomError::SwapAlreadyApproved
    );

    // Check if proposal is still active (not cancelled by threshold vote)
    require!(
        vault.pending_transactions.contains(&swap_transaction.id),
        CustomError::ProposalCancelled
    );

    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp < swap_transaction.expires_at,
        CustomError::SwapExpired
    );

    swap_transaction.approvals.push(signer.key());

    Ok(())
}
