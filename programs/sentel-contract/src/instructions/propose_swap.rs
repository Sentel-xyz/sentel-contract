use crate::{contexts::ProposeSwap, errors::CustomError, state::SwapTransactionState};
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

pub fn propose_swap(
    ctx: Context<ProposeSwap>,
    input_mint: Pubkey,
    output_mint: Pubkey,
    input_amount: u64,
    minimum_output_amount: u64,
    _vault_id: u64,
    _creator: Pubkey,
) -> Result<()> {
    require!(input_amount > 0, CustomError::InvalidAmount);
    require!(minimum_output_amount > 0, CustomError::InvalidAmount);

    let vault = &mut ctx.accounts.vault;
    let swap_transaction = &mut ctx.accounts.swap_transaction;

    require!(
        vault.owners.contains(&ctx.accounts.proposer.key()),
        CustomError::UnauthorizedProposer
    );

    require!(
        vault.pending_transactions.len() < 50,
        CustomError::TooManyPendingTransactions
    );

    // The last remaining account is the vault's input token account (SPL Token program owned).
    // Only the accounts before it are pending transaction PDAs owned by this program.
    // C-1: Enforce program ownership on all PDA accounts, but not on the token account.
    let pending_accounts = if !ctx.remaining_accounts.is_empty() {
        &ctx.remaining_accounts[..ctx.remaining_accounts.len() - 1]
    } else {
        ctx.remaining_accounts
    };

    let mut locked_token_amounts: std::collections::HashMap<Pubkey, u64> =
        std::collections::HashMap::new();

    for account_info in pending_accounts.iter() {
        require!(
            account_info.owner == ctx.program_id,
            CustomError::InvalidAccount
        );

        if let Ok(pending_tx) =
            crate::state::TransactionState::try_deserialize(&mut &account_info.data.borrow()[..])
        {
            if !pending_tx.executed && pending_tx.token_type {
                let current = locked_token_amounts.entry(pending_tx.mint).or_insert(0);
                *current = current
                    .checked_add(pending_tx.amount)
                    .ok_or(CustomError::InsufficientFunds)?;
            }
        } else if let Ok(pending_swap) =
            SwapTransactionState::try_deserialize(&mut &account_info.data.borrow()[..])
        {
            if !pending_swap.executed {
                let current = locked_token_amounts
                    .entry(pending_swap.input_mint)
                    .or_insert(0);
                *current = current
                    .checked_add(pending_swap.input_amount)
                    .ok_or(CustomError::InsufficientFunds)?;
            }
        }
    }

    // Token balance check is mandatory  fail hard if no valid token account is provided.
    let last_account = ctx
        .remaining_accounts
        .last()
        .ok_or(CustomError::InvalidTokenAccount)?;

    let token_account = TokenAccount::try_deserialize(&mut &last_account.data.borrow()[..])
        .map_err(|_| CustomError::InvalidTokenAccount)?;

    require!(
        token_account.owner == vault.key() && token_account.mint == input_mint,
        CustomError::InvalidTokenAccount
    );

    let current_locked = locked_token_amounts.get(&input_mint).unwrap_or(&0);
    let available_balance = token_account
        .amount
        .checked_sub(*current_locked)
        .ok_or(CustomError::InsufficientAvailableBalance)?;

    require!(
        available_balance >= input_amount,
        CustomError::InsufficientAvailableBalance
    );

    let clock = Clock::get()?;

    swap_transaction.id = vault.nonce;
    swap_transaction.proposer = ctx.accounts.proposer.key();
    swap_transaction.input_mint = input_mint;
    swap_transaction.output_mint = output_mint;
    swap_transaction.input_amount = input_amount;
    swap_transaction.minimum_output_amount = minimum_output_amount;
    swap_transaction.approvals = Vec::new();
    swap_transaction.cancellations = Vec::new();
    swap_transaction.executed = false;
    swap_transaction.created_at = clock.unix_timestamp;
    swap_transaction.expires_at = clock
        .unix_timestamp
        .checked_add(crate::TRANSACTION_EXPIRY_SECONDS)
        .ok_or(CustomError::InvalidAmount)?;

    vault.pending_transactions.push(swap_transaction.id);
    vault.nonce = vault
        .nonce
        .checked_add(1)
        .ok_or(CustomError::InvalidAmount)?;

    Ok(())
}
