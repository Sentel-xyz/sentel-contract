use crate::contexts::*;
use crate::TransactionStatusEvent;
use anchor_lang::prelude::*;

pub fn get_transaction_status(
    ctx: Context<GetTransactionStatus>,
    _creator: Pubkey,
    _vault_id: u64,
    _transaction_nonce: u64,
) -> Result<()> {
    let vault = &ctx.accounts.vault;
    let transaction = &ctx.accounts.transaction;

    emit!(TransactionStatusEvent {
        transaction_id: transaction.id,
        target: transaction.target,
        amount: transaction.amount,
        mint: transaction.mint,
        approvals: transaction.approvals.clone(),
        approval_count: transaction.approvals.len() as u8,
        required_approvals: vault.threshold,
        executed: transaction.executed,
        token_type: transaction.token_type,
    });

    Ok(())
}
