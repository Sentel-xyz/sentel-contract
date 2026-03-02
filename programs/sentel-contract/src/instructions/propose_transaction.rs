use crate::{contexts::ProposeTransaction, errors::CustomError, state::TransactionState};
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

pub fn propose_transaction(
    ctx: Context<ProposeTransaction>,
    receiver: Pubkey,
    token_type: bool,
    mint: Pubkey,
    amount: u64,
    _vault_id: u64,
    _creator: Pubkey,
) -> Result<()> {
    require!(amount > 0, CustomError::InvalidAmount);

    let vault = &mut ctx.accounts.vault;
    let transaction = &mut ctx.accounts.transaction;

    require!(
        vault.owners.contains(&ctx.accounts.proposer.key()),
        CustomError::UnauthorizedProposer
    );

    require!(
        vault.pending_transactions.len() < 50,
        CustomError::TooManyPendingTransactions
    );

    // Accumulate SOL and token amounts that are already locked in pending transactions.
    // Determine which remaining accounts are pending transaction PDAs vs. the optional token account.
    // The optional vault token account (for token_type = true balance checks) is always the last
    // remaining account and is owned by the SPL Token program, not this program.
    let pending_tx_accounts = if token_type && !ctx.remaining_accounts.is_empty() {
        &ctx.remaining_accounts[..ctx.remaining_accounts.len() - 1]
    } else {
        ctx.remaining_accounts
    };

    // C-1: Every account treated as a pending transaction PDA must be owned by this program.
    let mut locked_sol_amount: u64 = 0;
    let mut locked_token_amounts: std::collections::HashMap<Pubkey, u64> =
        std::collections::HashMap::new();

    for account_info in pending_tx_accounts.iter() {
        require!(
            account_info.owner == ctx.program_id,
            CustomError::InvalidAccount
        );

        if let Ok(pending_tx) =
            TransactionState::try_deserialize(&mut &account_info.data.borrow()[..])
        {
            if !pending_tx.executed {
                if pending_tx.token_type {
                    let current = locked_token_amounts.entry(pending_tx.mint).or_insert(0);
                    *current = current
                        .checked_add(pending_tx.amount)
                        .ok_or(CustomError::InsufficientFunds)?;
                } else {
                    let fee_amount = if pending_tx.amount > 0 {
                        let calculated_fee =
                            pending_tx.amount * crate::PROTOCOL_FEE_BASIS_POINTS / 10000;
                        calculated_fee.clamp(crate::MIN_FEE_LAMPORTS, crate::MAX_FEE_LAMPORTS)
                    } else {
                        0
                    };
                    locked_sol_amount = locked_sol_amount
                        .checked_add(pending_tx.amount)
                        .and_then(|v| v.checked_add(fee_amount))
                        .ok_or(CustomError::InsufficientFunds)?;
                }
            }
        }
    }

    if token_type {
        // H-3: Token balance check is mandatory  fail hard if no valid token account is provided.
        let last_account = ctx
            .remaining_accounts
            .last()
            .ok_or(CustomError::InvalidTokenAccount)?;

        let token_account = TokenAccount::try_deserialize(&mut &last_account.data.borrow()[..])
            .map_err(|_| CustomError::InvalidTokenAccount)?;

        require!(
            token_account.owner == vault.key() && token_account.mint == mint,
            CustomError::InvalidTokenAccount
        );

        let current_locked = locked_token_amounts.get(&mint).unwrap_or(&0);
        let available_balance = token_account
            .amount
            .checked_sub(*current_locked)
            .ok_or(CustomError::InsufficientAvailableBalance)?;

        require!(
            available_balance >= amount,
            CustomError::InsufficientAvailableBalance
        );

        // SOL for the protocol fee is still required even for token sends.
        let token_fee = crate::MIN_FEE_LAMPORTS;
        let vault_sol_balance = vault.to_account_info().lamports();
        let available_sol = vault_sol_balance
            .checked_sub(locked_sol_amount)
            .ok_or(CustomError::InsufficientAvailableBalance)?;

        require!(
            available_sol >= token_fee,
            CustomError::InsufficientFundsForFee
        );
    } else {
        let vault_info = vault.to_account_info();
        let vault_balance = vault_info.lamports();
        let rent_exempt = Rent::get()?.minimum_balance(vault_info.data_len());
        // Subtract rent-exempt reserve so the vault cannot be made non-rent-exempt by a proposal.
        let spendable_balance = vault_balance.saturating_sub(rent_exempt);

        let fee_amount = if amount > 0 {
            let calculated_fee = amount * crate::PROTOCOL_FEE_BASIS_POINTS / 10000;
            calculated_fee.clamp(crate::MIN_FEE_LAMPORTS, crate::MAX_FEE_LAMPORTS)
        } else {
            0
        };

        let total_needed = amount
            .checked_add(fee_amount)
            .ok_or(CustomError::InsufficientFunds)?;
        // Use spendable_balance (after rent reserve) as the ceiling, minus any locked SOL.
        let available_balance = spendable_balance
            .checked_sub(locked_sol_amount)
            .ok_or(CustomError::InsufficientAvailableBalance)?;

        require!(
            available_balance >= total_needed,
            CustomError::InsufficientAvailableBalance
        );
    }

    let clock = Clock::get()?;

    transaction.id = vault.nonce;
    transaction.proposer = ctx.accounts.proposer.key();
    transaction.target = receiver;
    transaction.amount = amount;
    transaction.mint = mint;
    transaction.approvals = Vec::new();
    transaction.cancellations = Vec::new();
    transaction.executed = false;
    transaction.token_type = token_type;
    transaction.created_at = clock.unix_timestamp;
    transaction.expires_at = clock
        .unix_timestamp
        .checked_add(crate::TRANSACTION_EXPIRY_SECONDS)
        .ok_or(CustomError::InvalidAmount)?;

    vault.pending_transactions.push(transaction.id);
    vault.nonce = vault
        .nonce
        .checked_add(1)
        .ok_or(CustomError::InvalidAmount)?;

    Ok(())
}
