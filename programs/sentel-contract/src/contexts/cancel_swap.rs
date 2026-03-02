use crate::state::{SwapTransactionState, VaultState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64, swap_nonce: u64)]
pub struct CancelSwap<'info> {
    #[account(
        mut,
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    // No `close`  -  account stays alive while cancel votes accumulate
    #[account(
        mut,
        seeds = [b"swap", vault.key().as_ref(), &swap_nonce.to_le_bytes()],
        bump
    )]
    pub swap_transaction: Account<'info, SwapTransactionState>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
