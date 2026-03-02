use crate::state::{SwapTransactionState, VaultState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64, swap_nonce: u64)]
pub struct ApproveSwap<'info> {
    #[account(
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"swap", vault.key().as_ref(), &swap_nonce.to_le_bytes()],
        bump
    )]
    pub swap_transaction: Account<'info, SwapTransactionState>,

    #[account(signer)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
