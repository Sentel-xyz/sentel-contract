use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    #[max_len(10)]
    pub owners: Vec<Pubkey>,
    pub threshold: u8,
    pub nonce: u64,
    #[max_len(50)]
    pub pending_transactions: Vec<u64>,
    #[max_len(20)]
    pub name: String,
}

#[account]
#[derive(InitSpace)]
pub struct TransactionState {
    pub id: u64,
    pub proposer: Pubkey,
    pub target: Pubkey,
    pub amount: u64,
    pub mint: Pubkey,
    #[max_len(10)]
    pub approvals: Vec<Pubkey>,
    /// Owners who have voted to cancel. Reaches threshold -> proposal is cancelled.
    #[max_len(10)]
    pub cancellations: Vec<Pubkey>,
    pub executed: bool,
    pub token_type: bool,
    pub created_at: i64,
    pub expires_at: i64,
}

#[account]
#[derive(InitSpace)]
pub struct SwapTransactionState {
    pub id: u64,
    pub proposer: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub input_amount: u64,
    pub minimum_output_amount: u64,
    #[max_len(10)]
    pub approvals: Vec<Pubkey>,
    /// Owners who have voted to cancel. Reaches threshold -> proposal is cancelled.
    #[max_len(10)]
    pub cancellations: Vec<Pubkey>,
    pub executed: bool,
    pub created_at: i64,
    pub expires_at: i64,
}

#[account]
#[derive(InitSpace)]
pub struct WrapTransactionState {
    pub id: u64,
    pub amount: u64,
    pub proposer: Pubkey,
    #[max_len(10)]
    pub approvals: Vec<Pubkey>,
    /// Owners who have voted to cancel. Reaches threshold -> proposal is cancelled.
    #[max_len(10)]
    pub cancellations: Vec<Pubkey>,
    pub executed: bool,
    pub created_at: i64,
    pub expires_at: i64,
}

/// Represents a token allocation in a balanced vault
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct TokenAllocation {
    pub mint: Pubkey,
    pub percentage: u16, // Basis points (0-10000, where 10000 = 100%)
}

/// Balanced vault that automatically rebalances token holdings
#[account]
#[derive(InitSpace)]
pub struct BalancedVaultState {
    pub creator: Pubkey,
    pub vault_id: u64,
    #[max_len(10)]
    pub owners: Vec<Pubkey>,
    pub threshold: u8,
    #[max_len(10)]
    pub allocations: Vec<TokenAllocation>,
    pub is_active: bool,
    pub created_at: i64,
    #[max_len(20)]
    pub name: String,
    pub bump: u8,
    pub nonce: u64,
    #[max_len(10)]
    pub pending_transactions: Vec<u64>,
}

/// Rebalance proposal for balanced vaults - multisig gate for the rebalance_vault instruction
#[account]
#[derive(InitSpace)]
pub struct RebalanceProposalState {
    pub id: u64,
    pub proposer: Pubkey,
    #[max_len(10)]
    pub approvals: Vec<Pubkey>,
    /// Owners who have voted to cancel. Reaches threshold -> proposal is cancelled.
    #[max_len(10)]
    pub cancellations: Vec<Pubkey>,
    pub executed: bool,
    pub created_at: i64,
    pub expires_at: i64,
    /// Number of individual swaps that have been executed so far (used by execute_rebalance_swap).
    pub swaps_executed: u32,
}

/// Retrieve transaction for balanced vaults - swaps all tokens to WSOL and sends to recipient
#[account]
#[derive(InitSpace)]
pub struct RetrieveTransactionState {
    pub id: u64,
    pub proposer: Pubkey,
    pub recipient: Pubkey,
    #[max_len(10)]
    pub approvals: Vec<Pubkey>,
    /// Owners who have voted to cancel. Reaches threshold -> proposal is cancelled.
    #[max_len(10)]
    pub cancellations: Vec<Pubkey>,
    pub executed: bool,
    pub created_at: i64,
    pub expires_at: i64,
}
