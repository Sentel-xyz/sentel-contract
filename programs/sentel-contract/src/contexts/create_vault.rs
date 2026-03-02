use crate::state::VaultState;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(owners: Vec<Pubkey>, threshold: u8, vault_id: u64)]
pub struct CreateVaultAccount<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + VaultState::INIT_SPACE,
        seeds = [b"vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump,
    )]
    pub vault: Account<'info, VaultState>,

    #[account(mut, signer)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}
