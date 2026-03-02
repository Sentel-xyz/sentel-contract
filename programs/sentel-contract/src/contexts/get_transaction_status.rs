use crate::state::{TransactionState, VaultState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64, transaction_nonce: u64)]
pub struct GetTransactionStatus<'info> {
    #[account(
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    #[account(
        seeds = [b"transaction", vault.key().as_ref(), &transaction_nonce.to_le_bytes()],
        bump
    )]
    pub transaction: Account<'info, TransactionState>,
}
