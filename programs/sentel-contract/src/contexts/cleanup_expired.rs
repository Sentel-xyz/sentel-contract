use crate::state::{TransactionState, VaultState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64, transaction_nonce: u64)]
pub struct CleanupExpired<'info> {
    #[account(
        mut,
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"transaction", vault.key().as_ref(), &transaction_nonce.to_le_bytes()],
        bump,
        close = rent_receiver
    )]
    pub transaction: Account<'info, TransactionState>,

    #[account(mut)]
    pub rent_receiver: Signer<'info>,

    pub system_program: Program<'info, System>,
}
