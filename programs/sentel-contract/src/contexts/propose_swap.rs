use crate::state::{SwapTransactionState, VaultState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(input_mint: Pubkey, output_mint: Pubkey, input_amount: u64, minimum_output_amount: u64, vault_id: u64, creator: Pubkey)]
pub struct ProposeSwap<'info> {
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
        space = 8 + SwapTransactionState::INIT_SPACE,
        seeds = [b"swap", vault.key().as_ref(), &vault.nonce.to_le_bytes()],
        bump
    )]
    pub swap_transaction: Account<'info, SwapTransactionState>,

    #[account(mut)]
    pub proposer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
