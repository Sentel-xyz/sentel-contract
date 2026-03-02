use crate::state::BalancedVaultState;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct UpdateAllocations<'info> {
    #[account(
        mut,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    pub creator: SystemAccount<'info>,

    #[account(mut)]
    pub updater: Signer<'info>,
}
