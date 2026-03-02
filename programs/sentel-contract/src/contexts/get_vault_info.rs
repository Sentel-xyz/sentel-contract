use crate::state::VaultState;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64)]
pub struct GetVaultInfo<'info> {
    #[account(
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,
}
