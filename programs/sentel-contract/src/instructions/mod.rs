pub mod approve_retrieve_transaction;
pub mod approve_swap;
pub mod approve_transaction;
pub mod approve_wrap;
pub mod cancel_retrieve_transaction;
pub mod cancel_swap;
pub mod cancel_transaction;
pub mod cancel_wrap;
pub mod cleanup_expired;
pub mod close_balanced_vault;
pub mod close_vault;
pub mod close_zombie_retrieve;
pub mod create_vault;
pub mod execute_retrieve_transaction;
pub mod execute_swap;
pub mod execute_transaction;
pub mod execute_wrap;
pub mod get_transaction_status;
pub mod get_vault_info;
pub mod open_balanced_vault;
pub mod propose_retrieve_transaction;
pub mod propose_sol_swap;
pub mod propose_swap;
pub mod propose_transaction;
pub mod propose_wrap;
pub mod rebalance_vault;
pub mod swap_token_to_wsol;
pub mod unwrap_wsol_for_rebalance;
pub mod update_allocations;
pub mod wrap_sol_for_rebalance;

pub use approve_retrieve_transaction::*;
pub use approve_swap::*;
pub use approve_transaction::*;
pub use approve_wrap::*;
pub use cancel_retrieve_transaction::*;
pub use cancel_swap::*;
pub use cancel_transaction::*;
pub use cancel_wrap::*;
pub use cleanup_expired::*;
pub use close_balanced_vault::*;
pub use close_vault::*;
pub use close_zombie_retrieve::*;
pub use create_vault::*;
pub use execute_retrieve_transaction::*;
pub use execute_swap::*;
pub use execute_transaction::*;
pub use execute_wrap::*;
pub use get_transaction_status::*;
pub use get_vault_info::*;
pub use open_balanced_vault::*;
pub use propose_retrieve_transaction::*;
pub use propose_sol_swap::*;
pub use propose_swap::*;
pub use propose_transaction::*;
pub use propose_wrap::*;
pub use rebalance_vault::*;
pub use swap_token_to_wsol::*;
pub use unwrap_wsol_for_rebalance::*;
pub use update_allocations::*;
pub use wrap_sol_for_rebalance::*;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_lang::solana_program::instruction::AccountMeta;

/// Build a Jupiter AccountMeta for a remaining account, marking the vault PDA as signer.
pub(crate) fn jupiter_account_meta(acc: &AccountInfo, vault_key: &Pubkey) -> AccountMeta {
    let is_vault = acc.key == vault_key;
    if acc.is_writable {
        AccountMeta::new(*acc.key, is_vault)
    } else {
        AccountMeta::new_readonly(*acc.key, is_vault)
    }
}
