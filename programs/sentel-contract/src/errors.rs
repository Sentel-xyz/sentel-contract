use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("Invalid threshold - must be > 0 and <= number of owners")]
    InvalidThreshold,
    #[msg("Owners list cannot be empty")]
    EmptyOwners,
    #[msg("Amount must be greater than 0")]
    InvalidAmount,
    #[msg("Only vault owners can propose transactions")]
    UnauthorizedProposer,
    #[msg("Signer has already approved this transaction")]
    AlreadyApproved,
    #[msg("Transaction has already been executed")]
    TransactionAlreadyExecuted,
    #[msg("Target account does not match transaction target")]
    InvalidTarget,
    #[msg("Missing required token accounts for SPL token transfer")]
    MissingTokenAccounts,
    #[msg("Token account mint does not match transaction mint")]
    InvalidMint,
    #[msg("Token account is not owned by the vault")]
    InvalidTokenAccountOwner,
    #[msg("Missing token account")]
    MissingTokenAccount,
    #[msg("Missing token program")]
    MissingTokenProgram,
    #[msg("Invalid token account")]
    InvalidTokenAccount,
    #[msg("Insufficient funds in vault for fee")]
    InsufficientFundsForFee,
    #[msg("Transaction not found")]
    TransactionNotFound,
    #[msg("Insufficient funds in vault for this transaction")]
    InsufficientFunds,
    #[msg("Balance verification failed - potential attack detected")]
    BalanceVerificationFailed,
    #[msg("Insufficient approvals to execute transaction")]
    InsufficientApprovals,
    #[msg("Duplicate Owner")]
    DuplicateOwner,
    #[msg("Duplicate mint in allocation")]
    DuplicateMint,
    #[msg("Too many owners - maximum of 10 allowed")]
    TooManyOwners,
    #[msg("Transaction has expired")]
    TransactionExpired,
    #[msg("Too many pending transactions - maximum of 50 allowed")]
    TooManyPendingTransactions,
    #[msg("Transaction has not expired yet")]
    TransactionNotExpired,
    #[msg("Vault name is too long - maximum of 20 characters allowed")]
    NameTooLong,
    #[msg("Vault name cannot be empty")]
    EmptyName,
    #[msg("Invalid fee recipient address")]
    InvalidFeeRecipient,
    #[msg("Vault has pending transactions and cannot be closed")]
    VaultHasPendingTransactions,
    #[msg("Vault balance is too high for closure - must be less than 0.3 SOL")]
    VaultBalanceTooHighForClosure,
    #[msg("Only the vault creator can close the vault")]
    UnauthorizedCreator,
    #[msg("Vault still contains SPL tokens and cannot be closed")]
    VaultHasTokenBalance,
    #[msg("Insufficient available balance - funds are locked in pending transactions")]
    InsufficientAvailableBalance,
    #[msg("Swap transaction not found")]
    SwapTransactionNotFound,
    #[msg("Swap transaction has already been executed")]
    SwapAlreadyExecuted,
    #[msg("Swap transaction has expired")]
    SwapExpired,
    #[msg("Swap transaction has not expired yet")]
    SwapNotExpired,
    #[msg("Insufficient approvals to execute swap")]
    InsufficientApprovalsForSwap,
    #[msg("Signer has already approved this swap")]
    SwapAlreadyApproved,
    #[msg("Invalid token allocation - total must equal 100%")]
    InvalidAllocationTotal,
    #[msg("Too many token allocations - maximum of 10 allowed")]
    TooManyAllocations,
    #[msg("Balanced vault is not active")]
    BalancedVaultNotActive,
    #[msg("Invalid percentage - must be between 0 and 10000 basis points")]
    InvalidPercentage,
    #[msg("Balanced vault already exists for this ID")]
    BalancedVaultAlreadyExists,
    #[msg("Cannot rebalance with zero WSOL")]
    InsufficientWsolForRebalance,
    #[msg("Minimum output amount not met")]
    MinimumOutputNotMet,
    #[msg("Invalid nonce - does not match current vault nonce")]
    InvalidNonce,
    #[msg("Unauthorized - not a vault owner")]
    Unauthorized,
    #[msg("A retrieval is already pending - cancel or execute it before proposing a new one")]
    RetrievalAlreadyPending,
    #[msg("Account is not owned by this program")]
    InvalidAccount,
    #[msg("Signer has already voted to cancel this proposal")]
    AlreadyCancelledVote,
    #[msg("Proposal has been cancelled by threshold vote")]
    ProposalCancelled,
}
