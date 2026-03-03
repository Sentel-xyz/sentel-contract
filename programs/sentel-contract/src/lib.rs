use anchor_lang::prelude::*;

pub mod contexts;
pub mod errors;
pub mod instructions;
pub mod state;

use contexts::*;

declare_id!("Engn3cBYZPvP37myVuiwanqhs2omZxWRS7twNRJX8uZV");

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Sentel",
    project_url: "https://sentel.xyz",
    contacts: "email:sentel_xyz@proton.me",
    policy: "https://sentel.xyz/legal/security",
    source_code: "https://github.com/Sentel-xyz/sentel-contract",
    auditors: "None"
}

pub const PROTOCOL_FEE_RECIPIENT: &str = "BdXd6EzjCFhLmMDF1D2vm2zDrPuCzfHxyAezvPMudaU8";
pub const PROTOCOL_FEE_BASIS_POINTS: u64 = 5;
pub const MIN_FEE_LAMPORTS: u64 = 5_000_000;
pub const MAX_FEE_LAMPORTS: u64 = 200_000_000;
pub const TRANSACTION_EXPIRY_SECONDS: i64 = 604_800;
pub const JUPITER_V6_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

#[event]
pub struct VaultInfoEvent {
    pub vault_address: Pubkey,
    pub owners: Vec<Pubkey>,
    pub threshold: u8,
    pub balance: u64,
    pub nonce: u64,
    pub pending_transactions_count: u64,
    pub name: String,
}

#[event]
pub struct TransactionStatusEvent {
    pub transaction_id: u64,
    pub target: Pubkey,
    pub amount: u64,
    pub mint: Pubkey,
    pub approvals: Vec<Pubkey>,
    pub approval_count: u8,
    pub required_approvals: u8,
    pub executed: bool,
    pub token_type: bool,
}

#[event]
pub struct TransactionCancelledEvent {
    pub transaction_id: u64,
    pub cancelled_by: Pubkey,
    pub vault: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct VaultClosedEvent {
    pub vault_address: Pubkey,
    pub creator: Pubkey,
    pub final_balance: u64,
    pub timestamp: i64,
}

#[program]
pub mod sentel_contract {
    use super::*;

    pub fn create_vault(
        ctx: Context<CreateVaultAccount>,
        owners: Vec<Pubkey>,
        threshold: u8,
        vault_id: u64,
        name: String,
    ) -> Result<()> {
        instructions::create_vault(ctx, owners, threshold, vault_id, name)
    }

    pub fn propose_transaction(
        ctx: Context<ProposeTransaction>,
        receiver: Pubkey,
        token_type: bool,
        mint: Pubkey,
        amount: u64,
        vault_id: u64,
        creator: Pubkey,
    ) -> Result<()> {
        instructions::propose_transaction(
            ctx, receiver, token_type, mint, amount, vault_id, creator,
        )
    }

    pub fn approve_transaction(
        ctx: Context<ApproveTransaction>,
        creator: Pubkey,
        vault_id: u64,
        transaction_nonce: u64,
    ) -> Result<()> {
        instructions::approve_transaction(ctx, creator, vault_id, transaction_nonce)
    }

    pub fn execute_transaction(
        ctx: Context<ExecuteTransaction>,
        creator: Pubkey,
        vault_id: u64,
        transaction_nonce: u64,
    ) -> Result<()> {
        instructions::execute_transaction(ctx, creator, vault_id, transaction_nonce)
    }

    pub fn get_vault_info(
        ctx: Context<GetVaultInfo>,
        creator: Pubkey,
        vault_id: u64,
    ) -> Result<()> {
        instructions::get_vault_info(ctx, creator, vault_id)
    }

    pub fn get_transaction_status(
        ctx: Context<GetTransactionStatus>,
        creator: Pubkey,
        vault_id: u64,
        transaction_nonce: u64,
    ) -> Result<()> {
        instructions::get_transaction_status(ctx, creator, vault_id, transaction_nonce)
    }

    pub fn cleanup_expired(
        ctx: Context<CleanupExpired>,
        creator: Pubkey,
        vault_id: u64,
        transaction_nonce: u64,
    ) -> Result<()> {
        instructions::cleanup_expired(ctx, creator, vault_id, transaction_nonce)
    }

    pub fn cancel_transaction(
        ctx: Context<CancelTransaction>,
        creator: Pubkey,
        vault_id: u64,
        transaction_nonce: u64,
    ) -> Result<()> {
        instructions::cancel_transaction(ctx, creator, vault_id, transaction_nonce)
    }

    pub fn close_vault<'info>(
        ctx: Context<'_, '_, '_, 'info, CloseVault<'info>>,
        creator_key: Pubkey,
        vault_id: u64,
    ) -> Result<()> {
        instructions::close_vault(ctx, creator_key, vault_id)
    }

    pub fn propose_swap(
        ctx: Context<ProposeSwap>,
        input_mint: Pubkey,
        output_mint: Pubkey,
        input_amount: u64,
        minimum_output_amount: u64,
        vault_id: u64,
        creator: Pubkey,
    ) -> Result<()> {
        instructions::propose_swap(
            ctx,
            input_mint,
            output_mint,
            input_amount,
            minimum_output_amount,
            vault_id,
            creator,
        )
    }

    /// Propose a swap from native SOL to any SPL token.
    /// The SOL is atomically wrapped to WSOL (with protocol fee) at propose-time.
    /// After approval, call `execute_swap` with WSOL as the input mint.
    pub fn propose_sol_swap(
        ctx: Context<ProposeSolSwap>,
        output_mint: Pubkey,
        sol_amount: u64,
        minimum_output_amount: u64,
        vault_id: u64,
        creator: Pubkey,
    ) -> Result<()> {
        instructions::propose_sol_swap(
            ctx,
            output_mint,
            sol_amount,
            minimum_output_amount,
            vault_id,
            creator,
        )
    }

    pub fn approve_swap(
        ctx: Context<ApproveSwap>,
        creator: Pubkey,
        vault_id: u64,
        swap_nonce: u64,
    ) -> Result<()> {
        instructions::approve_swap(ctx, creator, vault_id, swap_nonce)
    }

    pub fn execute_swap<'info>(
        ctx: Context<'_, '_, '_, 'info, ExecuteSwap<'info>>,
        creator: Pubkey,
        vault_id: u64,
        swap_nonce: u64,
        jupiter_instruction_data: Vec<u8>,
    ) -> Result<()> {
        instructions::execute_swap(ctx, creator, vault_id, swap_nonce, jupiter_instruction_data)
    }

    pub fn cancel_swap(
        ctx: Context<CancelSwap>,
        creator: Pubkey,
        vault_id: u64,
        swap_nonce: u64,
    ) -> Result<()> {
        instructions::cancel_swap(ctx, creator, vault_id, swap_nonce)
    }

    pub fn propose_wrap(
        ctx: Context<ProposeWrap>,
        amount: u64,
        vault_id: u64,
        creator: Pubkey,
    ) -> Result<()> {
        instructions::propose_wrap(ctx, amount, vault_id, creator)
    }

    pub fn approve_wrap(
        ctx: Context<ApproveWrap>,
        creator: Pubkey,
        vault_id: u64,
        wrap_nonce: u64,
    ) -> Result<()> {
        instructions::approve_wrap(ctx, creator, vault_id, wrap_nonce)
    }

    pub fn execute_wrap(
        ctx: Context<ExecuteWrap>,
        creator: Pubkey,
        vault_id: u64,
        wrap_nonce: u64,
    ) -> Result<()> {
        instructions::execute_wrap(ctx, creator, vault_id, wrap_nonce)
    }

    pub fn cancel_wrap(
        ctx: Context<CancelWrap>,
        creator: Pubkey,
        vault_id: u64,
        wrap_nonce: u64,
    ) -> Result<()> {
        instructions::cancel_wrap(ctx, creator, vault_id, wrap_nonce)
    }

    // ============================================
    // Balanced Vault Instructions
    // ============================================

    pub fn open_balanced_vault(
        ctx: Context<OpenBalancedVault>,
        vault_id: u64,
        owners: Vec<Pubkey>,
        threshold: u8,
        allocations: Vec<state::TokenAllocation>,
        name: String,
    ) -> Result<()> {
        instructions::open_balanced_vault(ctx, vault_id, owners, threshold, allocations, name)
    }

    pub fn close_balanced_vault(ctx: Context<CloseBalancedVault>, vault_id: u64) -> Result<()> {
        instructions::close_balanced_vault(ctx, vault_id)
    }

    pub fn rebalance_vault<'info>(
        ctx: Context<'_, '_, '_, 'info, RebalanceVault<'info>>,
        vault_id: u64,
        jupiter_swap_data: Vec<Vec<u8>>,
        swap_account_counts: Vec<u32>,
    ) -> Result<()> {
        instructions::rebalance_vault(ctx, vault_id, jupiter_swap_data, swap_account_counts)
    }

    pub fn wrap_sol_for_rebalance(ctx: Context<WrapSolForRebalance>, vault_id: u64) -> Result<()> {
        instructions::wrap_sol_for_rebalance(ctx, vault_id)
    }

    pub fn unwrap_wsol_for_rebalance(
        ctx: Context<UnwrapWsolForRebalance>,
        vault_id: u64,
    ) -> Result<()> {
        instructions::unwrap_wsol_for_rebalance(ctx, vault_id)
    }

    pub fn update_allocations(
        ctx: Context<UpdateAllocations>,
        vault_id: u64,
        new_allocations: Vec<state::TokenAllocation>,
    ) -> Result<()> {
        instructions::update_allocations(ctx, vault_id, new_allocations)
    }

    pub fn swap_token_to_wsol<'info>(
        ctx: Context<'_, '_, '_, 'info, SwapTokenToWsol<'info>>,
        vault_id: u64,
        jupiter_swap_data: Vec<u8>,
        swap_account_count: u32,
    ) -> Result<()> {
        instructions::swap_token_to_wsol(ctx, vault_id, jupiter_swap_data, swap_account_count)
    }

    pub fn propose_retrieve_transaction(
        ctx: Context<ProposeRetrieveTransaction>,
        vault_id: u64,
        transaction_nonce: u64,
        recipient: Pubkey,
    ) -> Result<()> {
        instructions::propose_retrieve_transaction(ctx, vault_id, transaction_nonce, recipient)
    }

    pub fn approve_retrieve_transaction(
        ctx: Context<ApproveRetrieveTransaction>,
        vault_id: u64,
        transaction_nonce: u64,
    ) -> Result<()> {
        instructions::approve_retrieve_transaction(ctx, vault_id, transaction_nonce)
    }

    pub fn cancel_retrieve_transaction(
        ctx: Context<CancelRetrieveTransaction>,
        vault_id: u64,
        transaction_nonce: u64,
    ) -> Result<()> {
        instructions::cancel_retrieve_transaction(ctx, vault_id, transaction_nonce)
    }

    pub fn execute_retrieve_transaction<'info>(
        ctx: Context<'_, '_, '_, 'info, ExecuteRetrieveTransaction<'info>>,
        vault_id: u64,
        transaction_nonce: u64,
        jupiter_swap_data: Vec<Vec<u8>>,
        swap_account_counts: Vec<u32>,
    ) -> Result<()> {
        instructions::execute_retrieve_transaction(
            ctx,
            vault_id,
            transaction_nonce,
            jupiter_swap_data,
            swap_account_counts,
        )
    }

    /// Close an already-executed retrieve transaction PDA that was never reclaimed.
    /// Any vault owner can call this to unblock future proposals.
    pub fn close_zombie_retrieve(
        ctx: Context<CloseZombieRetrieve>,
        vault_id: u64,
        transaction_nonce: u64,
    ) -> Result<()> {
        instructions::close_zombie_retrieve(ctx, vault_id, transaction_nonce)
    }

    // ============================================
    // Rebalance Proposal Instructions (multisig)
    // ============================================

    pub fn propose_rebalance(
        ctx: Context<ProposeRebalance>,
        vault_id: u64,
        proposal_nonce: u64,
    ) -> Result<()> {
        instructions::propose_rebalance(ctx, vault_id, proposal_nonce)
    }

    pub fn approve_rebalance(
        ctx: Context<ApproveRebalance>,
        vault_id: u64,
        proposal_nonce: u64,
    ) -> Result<()> {
        instructions::approve_rebalance(ctx, vault_id, proposal_nonce)
    }

    pub fn cancel_rebalance(
        ctx: Context<CancelRebalance>,
        vault_id: u64,
        proposal_nonce: u64,
    ) -> Result<()> {
        instructions::cancel_rebalance(ctx, vault_id, proposal_nonce)
    }

    pub fn execute_rebalance<'info>(
        ctx: Context<'_, '_, '_, 'info, ExecuteRebalance<'info>>,
        vault_id: u64,
        proposal_nonce: u64,
        jupiter_swap_data: Vec<Vec<u8>>,
        swap_account_counts: Vec<u32>,
    ) -> Result<()> {
        instructions::execute_rebalance(
            ctx,
            vault_id,
            proposal_nonce,
            jupiter_swap_data,
            swap_account_counts,
        )
    }
}
