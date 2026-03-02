use crate::state::{BalancedVaultState, RetrieveTransactionState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(vault_id: u64, transaction_nonce: u64)]
pub struct ApproveRetrieveTransaction<'info> {
    #[account(
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    #[account(
        mut,
        seeds = [
            b"retrieve_transaction",
            creator.key().as_ref(),
            &vault_id.to_le_bytes(),
            &transaction_nonce.to_le_bytes()
        ],
        bump
    )]
    pub retrieve_transaction: Account<'info, RetrieveTransactionState>,

    pub creator: SystemAccount<'info>,

    pub approver: Signer<'info>,
}
