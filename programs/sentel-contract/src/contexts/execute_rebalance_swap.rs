use crate::state::{BalancedVaultState, RebalanceProposalState};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use std::str::FromStr;

#[derive(Accounts)]
#[instruction(vault_id: u64, proposal_nonce: u64)]
pub struct ExecuteRebalanceSwap<'info> {
    #[account(
        mut,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    /// The approved rebalance proposal. NOT closed here  stays open until finalize_rebalance.
    #[account(
        mut,
        seeds = [
            b"rebalance_proposal",
            creator.key().as_ref(),
            &vault_id.to_le_bytes(),
            &proposal_nonce.to_le_bytes()
        ],
        bump
    )]
    pub rebalance_proposal: Account<'info, RebalanceProposalState>,

    #[account(
        mut,
        constraint = vault_wsol_account.owner == balanced_vault.key() @ crate::errors::CustomError::InvalidTokenAccount,
        constraint = vault_wsol_account.mint == Pubkey::from_str(crate::WSOL_MINT).unwrap() @ crate::errors::CustomError::InvalidMint,
    )]
    pub vault_wsol_account: Account<'info, TokenAccount>,

    /// Creator is only used for PDA derivation.
    pub creator: SystemAccount<'info>,

    /// The vault owner who is executing the swap (must be an owner of balanced_vault).
    #[account(mut)]
    pub executor: Signer<'info>,

    /// CHECK: Jupiter V6 program - address validated in instruction against hardcoded constant.
    pub jupiter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}
