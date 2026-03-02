use crate::state::{VaultState, WrapTransactionState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64, wrap_nonce: u64)]
pub struct CancelWrap<'info> {
    #[account(
        mut,
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    // No `close`  -  account stays alive while cancel votes accumulate
    #[account(
        mut,
        seeds = [b"wrap", vault.key().as_ref(), &wrap_nonce.to_le_bytes()],
        bump
    )]
    pub wrap_transaction: Account<'info, WrapTransactionState>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
