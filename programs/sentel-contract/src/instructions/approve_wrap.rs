use crate::{contexts::ApproveWrap, errors::CustomError};
use anchor_lang::prelude::*;

pub fn approve_wrap(
    ctx: Context<ApproveWrap>,
    _creator: Pubkey,
    _vault_id: u64,
    _wrap_nonce: u64,
) -> Result<()> {
    let vault = &ctx.accounts.vault;
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

    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp <= wrap_transaction.expires_at,
        CustomError::TransactionExpired
    );

    // Reject duplicate approvals with an explicit error instead of silently ignoring them.
    require!(
        !wrap_transaction.approvals.contains(&signer.key()),
        CustomError::AlreadyApproved
    );

    wrap_transaction.approvals.push(signer.key());

    Ok(())
}
