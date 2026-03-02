use crate::state::{SwapTransactionState, VaultState};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use std::str::FromStr;

/// Proposes a swap from native SOL to an SPL token in a single transaction.
///
/// The SOL is atomically wrapped to WSOL (lamport transfer + protocol fee) here
/// at propose-time so that `execute_swap` can treat it as a normal WSOL->Token swap.
///
/// Anchor performs the `init` CPI to system_program first (before our handler code),
/// then our handler does only lamport mutations  no additional CPI after that, which
/// satisfies the Solana runtime constraint.
#[derive(Accounts)]
#[instruction(output_mint: Pubkey, sol_amount: u64, minimum_output_amount: u64, vault_id: u64, creator: Pubkey)]
pub struct ProposeSolSwap<'info> {
    #[account(
        mut,
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    /// The swap PDA  seeded on vault key + current nonce so every proposal is unique.
    #[account(
        init,
        payer = proposer,
        space = 8 + SwapTransactionState::INIT_SPACE,
        seeds = [b"swap", vault.key().as_ref(), &vault.nonce.to_le_bytes()],
        bump
    )]
    pub swap_transaction: Account<'info, SwapTransactionState>,

    /// The vault's WSOL ATA  must be owned by the vault PDA and use the canonical WSOL mint.
    #[account(
        mut,
        constraint = vault_wsol_account.owner == vault.key() @ crate::errors::CustomError::InvalidTokenAccount,
        constraint = vault_wsol_account.mint == Pubkey::from_str(crate::WSOL_MINT).unwrap() @ crate::errors::CustomError::InvalidMint,
    )]
    pub vault_wsol_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub proposer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
