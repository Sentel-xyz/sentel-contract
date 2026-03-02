use crate::state::{VaultState, WrapTransactionState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(amount: u64, vault_id: u64, creator: Pubkey)]
pub struct ProposeWrap<'info> {
    // Vault PDA seeded on creator so all owners derive the same canonical address.
    #[account(
        mut,
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    #[account(
        init,
        payer = proposer,
        space = 8 + WrapTransactionState::INIT_SPACE,
        seeds = [b"wrap", vault.key().as_ref(), &vault.nonce.to_le_bytes()],
        bump
    )]
    pub wrap_transaction: Account<'info, WrapTransactionState>,

    #[account(mut)]
    pub proposer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
