use crate::state::{WrapTransactionState, VaultState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64, wrap_nonce: u64)]
pub struct ApproveWrap<'info> {
    #[account(
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"wrap", vault.key().as_ref(), &wrap_nonce.to_le_bytes()],
        bump
    )]
    pub wrap_transaction: Account<'info, WrapTransactionState>,

    #[account(mut, signer)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
