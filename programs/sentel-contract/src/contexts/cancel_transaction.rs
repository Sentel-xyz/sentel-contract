use crate::state::{TransactionState, VaultState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64, transaction_nonce: u64)]
pub struct CancelTransaction<'info> {
    #[account(
        mut,
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    // NOTE: No `close` here  -  the PDA is closed by cleanup_expired or once fully cancelled
    // by a separate close instruction. The account stays alive while cancel votes accumulate.
    #[account(
        mut,
        seeds = [b"transaction", vault.key().as_ref(), &transaction_nonce.to_le_bytes()],
        bump
    )]
    pub transaction: Account<'info, TransactionState>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
