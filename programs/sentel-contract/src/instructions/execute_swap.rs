use crate::commitment::swap_commitment;
use crate::{contexts::ExecuteSwap, errors::CustomError};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token;
use std::str::FromStr;

pub fn execute_swap<'info>(
    ctx: Context<'_, '_, '_, 'info, ExecuteSwap<'info>>,
    creator: Pubkey,
    vault_id: u64,
    _swap_nonce: u64,
    jupiter_instruction_data: Vec<u8>,
) -> Result<()> {
    require!(
        ctx.accounts
            .vault
            .owners
            .contains(&ctx.accounts.signer.key()),
        CustomError::UnauthorizedProposer
    );

    let jupiter_program_id = Pubkey::from_str(crate::JUPITER_V6_PROGRAM_ID)
        .map_err(|_| CustomError::InvalidFeeRecipient)?;

    require!(
        ctx.accounts.jupiter_program.key() == jupiter_program_id,
        CustomError::InvalidFeeRecipient
    );

    require!(
        !ctx.accounts.swap_transaction.executed,
        CustomError::SwapAlreadyExecuted
    );

    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp < ctx.accounts.swap_transaction.expires_at,
        CustomError::SwapExpired
    );

    require!(
        ctx.accounts.swap_transaction.approvals.len() >= ctx.accounts.vault.threshold as usize,
        CustomError::InsufficientApprovalsForSwap
    );

    require!(
        ctx.accounts
            .vault
            .pending_transactions
            .contains(&ctx.accounts.swap_transaction.id),
        CustomError::ProposalCancelled
    );

    // The owners approved one specific route. Anything else is a different swap.
    require!(
        swap_commitment(&jupiter_instruction_data, ctx.remaining_accounts)
            == ctx.accounts.swap_transaction.payload_hash,
        CustomError::SwapPayloadMismatch
    );

    let wsol_mint = Pubkey::from_str(crate::WSOL_MINT).unwrap();
    let input_is_wsol = ctx.accounts.swap_transaction.input_mint == wsol_mint;
    let output_is_wsol = ctx.accounts.swap_transaction.output_mint == wsol_mint;
    let swap_id = ctx.accounts.swap_transaction.id;
    let input_amount = ctx.accounts.swap_transaction.input_amount;
    let minimum_output_amount = ctx.accounts.swap_transaction.minimum_output_amount;

    let vault_id_bytes = vault_id.to_le_bytes();
    let seeds = &[
        b"vault".as_ref(),
        creator.as_ref(),
        vault_id_bytes.as_ref(),
        &[ctx.bumps.vault],
    ];
    let signer_seeds = &[&seeds[..]];

    if input_is_wsol {
        token::sync_native(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::SyncNative {
                account: ctx.accounts.vault_input_token_account.to_account_info(),
            },
            signer_seeds,
        ))?;
    }

    // Snapshot after sync_native so a WSOL input reflects the wrapped lamports.
    ctx.accounts.vault_input_token_account.reload()?;
    ctx.accounts.vault_output_token_account.reload()?;
    let input_before = ctx.accounts.vault_input_token_account.amount;
    let output_before = ctx.accounts.vault_output_token_account.amount;

    let vault_key = ctx.accounts.vault.key();
    let jupiter_accounts: Vec<_> = ctx
        .remaining_accounts
        .iter()
        .map(|acc| crate::instructions::jupiter_account_meta(acc, &vault_key))
        .collect();

    let jupiter_ix = Instruction {
        program_id: jupiter_program_id,
        accounts: jupiter_accounts,
        data: jupiter_instruction_data,
    };

    invoke_signed(&jupiter_ix, ctx.remaining_accounts, signer_seeds)?;

    // Verify the economics the owners approved actually held, rather than trusting
    // the route to have been honest about them.
    ctx.accounts.vault_input_token_account.reload()?;
    ctx.accounts.vault_output_token_account.reload()?;

    let input_spent = input_before.saturating_sub(ctx.accounts.vault_input_token_account.amount);
    let output_gained = ctx
        .accounts
        .vault_output_token_account
        .amount
        .saturating_sub(output_before);

    require!(input_spent <= input_amount, CustomError::SwapInputExceeded);
    require!(
        output_gained >= minimum_output_amount,
        CustomError::SwapOutputBelowMinimum
    );

    {
        let vault = &mut ctx.accounts.vault;
        let swap_transaction = &mut ctx.accounts.swap_transaction;

        swap_transaction.executed = true;
        vault.pending_transactions.retain(|&id| id != swap_id);
    }

    // Unwrap before the fee debit below. close_account returns the lamports through a
    // CPI; debiting first makes the runtime's balance check fire before that lands.
    if output_is_wsol {
        token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::CloseAccount {
                account: ctx.accounts.vault_output_token_account.to_account_info(),
                destination: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.vault.to_account_info(),
            },
            signer_seeds,
        ))?;
    }

    // 5 bps of the input, clamped, taken from the vault's native SOL balance.
    let raw_fee = input_amount
        .saturating_mul(crate::PROTOCOL_FEE_BASIS_POINTS)
        .checked_div(10_000)
        .unwrap_or(0);
    let fee_amount = raw_fee
        .max(crate::MIN_FEE_LAMPORTS)
        .min(crate::MAX_FEE_LAMPORTS);

    let vault_info = ctx.accounts.vault.to_account_info();
    let fee_recipient_info = ctx.accounts.fee_recipient.to_account_info();
    let vault_lamports = vault_info.lamports();

    require!(
        vault_lamports >= fee_amount,
        CustomError::InsufficientFundsForFee
    );

    **vault_info.try_borrow_mut_lamports()? = vault_lamports
        .checked_sub(fee_amount)
        .ok_or(CustomError::InsufficientFunds)?;
    **fee_recipient_info.try_borrow_mut_lamports()? = fee_recipient_info
        .lamports()
        .checked_add(fee_amount)
        .ok_or(CustomError::InsufficientFunds)?;

    Ok(())
}
