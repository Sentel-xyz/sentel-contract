use crate::state::{BalancedVaultState, RetrieveTransactionState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(vault_id: u64, transaction_nonce: u64)]
pub struct CloseZombieRetrieve<'info> {
    #[account(
        mut,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    /// The already-executed (zombie) retrieve transaction PDA to reclaim.
    #[account(
        mut,
        seeds = [
            b"retrieve_transaction",
            creator.key().as_ref(),
            &vault_id.to_le_bytes(),
            &transaction_nonce.to_le_bytes()
        ],
        bump,
        close = rent_receiver
    )]
    pub retrieve_transaction: Account<'info, RetrieveTransactionState>,

    pub creator: SystemAccount<'info>,

    /// CHECK: Any vault owner calling this reclaims the rent.
    #[account(mut)]
    pub rent_receiver: Signer<'info>,

    pub system_program: Program<'info, System>,
}
