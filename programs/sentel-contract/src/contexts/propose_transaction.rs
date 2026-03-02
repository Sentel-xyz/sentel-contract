use crate::state::{TransactionState, VaultState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
// creator is placed last so the first positional args (receiver, token_type, mint, amount,
// vault_id) stay backward-compatible; creator is only needed for PDA derivation.
#[instruction(receiver: Pubkey, token_type: bool, mint: Pubkey, amount: u64, vault_id: u64, creator: Pubkey)]
pub struct ProposeTransaction<'info> {
    // Vault PDA is seeded on the creator so every vault owner shares the same canonical address.
    #[account(
        mut,
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    #[account(
        init,
        payer = proposer,
        space = 8 + TransactionState::INIT_SPACE,
        seeds = [b"transaction", vault.key().as_ref(), &vault.nonce.to_le_bytes()],
        bump
    )]
    pub transaction: Account<'info, TransactionState>,

    #[account(mut)]
    pub proposer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
