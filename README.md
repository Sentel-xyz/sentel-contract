# Sentel Contract

Anchor program powering the Sentel protocol on Solana. Two vault types:

- **Standard multisig vault** — holds SOL and SPL tokens, releases funds only after m-of-n owner approval.
- **Balanced vault** — holds a portfolio of SPL tokens at target allocations (basis points), rebalanced via Jupiter V6.

## Deployment

| Network      | Program ID                                     |
| ------------ | ---------------------------------------------- |
| Mainnet-beta | `Engn3cBYZPvP37myVuiwanqhs2omZxWRS7twNRJX8uZV` |

## How it works

Every value-moving action follows the same lifecycle:

```
propose -> approve (m-of-n) -> execute
```

Cancellation also requires a threshold vote — a single owner cannot unilaterally cancel a proposal.

Proposals expire after **7 days**. Expired proposals can be cleaned up by anyone to recover rent.

### Protocol fee

Collected on every execution, paid to a hard-coded on-chain recipient.

```
Fee           = amount * 0.05%
Min fee       = 0.005 SOL
Max fee       = 0.2 SOL
```

SPL token transfers and wraps apply the minimum fee floor. SOL transfers are proportional only.

## Instructions

### Standard vault

| Instruction              | Description                                                                                                             |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| `create_vault`           | Create a vault with a list of owners and an m-of-n threshold.                                                           |
| `propose_transaction`    | Propose a SOL or SPL token transfer.                                                                                    |
| `approve_transaction`    | Vote to approve a pending transfer.                                                                                     |
| `execute_transaction`    | Execute once the approval threshold is met.                                                                             |
| `cancel_transaction`     | Vote to cancel — removed from pending at threshold.                                                                     |
| `cleanup_expired`        | Reclaim rent from a proposal older than 7 days.                                                                         |
| `close_vault`            | Close the vault and return rent. Requires no pending transactions, all token accounts empty, and balance below 0.3 SOL. |
| `get_vault_info`         | Emit a `VaultInfoEvent` with current vault state.                                                                       |
| `get_transaction_status` | Emit a `TransactionStatusEvent` with current approval count.                                                            |
| `propose_swap`           | Propose an SPL-to-SPL Jupiter swap.                                                                                     |
| `propose_sol_swap`       | Propose a SOL-to-token swap. SOL is wrapped to WSOL atomically at propose-time.                                         |
| `approve_swap`           | Vote to approve a pending swap.                                                                                         |
| `execute_swap`           | Execute the swap via Jupiter V6 CPI.                                                                                    |
| `cancel_swap`            | Vote to cancel a pending swap.                                                                                          |
| `propose_wrap`           | Propose wrapping SOL to WSOL.                                                                                           |
| `approve_wrap`           | Vote to approve a pending wrap.                                                                                         |
| `execute_wrap`           | Move SOL to the vault WSOL ATA and collect the fee.                                                                     |
| `cancel_wrap`            | Vote to cancel a pending wrap.                                                                                          |

### Balanced vault

| Instruction                    | Description                                                                 |
| ------------------------------ | --------------------------------------------------------------------------- |
| `open_balanced_vault`          | Create a balanced vault with up to 10 token allocations (must sum to 100%). |
| `close_balanced_vault`         | Close the vault and return rent.                                            |
| `update_allocations`           | Update target allocations. Callable by any vault owner.                     |
| `wrap_sol_for_rebalance`       | Wrap vault SOL to WSOL before rebalancing.                                  |
| `unwrap_wsol_for_rebalance`    | Unwrap vault WSOL back to SOL.                                              |
| `rebalance_vault`              | Rebalance all positions via Jupiter V6.                                     |
| `swap_token_to_wsol`           | Swap a single token to WSOL.                                                |
| `propose_retrieve_transaction` | Propose full withdrawal to a recipient.                                     |
| `approve_retrieve_transaction` | Vote to approve a pending withdrawal.                                       |
| `cancel_retrieve_transaction`  | Vote to cancel a pending withdrawal.                                        |
| `execute_retrieve_transaction` | Liquidate all positions to SOL and send to recipient.                       |
| `close_zombie_retrieve`        | Reclaim rent from an already-executed retrieve PDA.                         |

## Build and test

Requires Rust `1.89.0`, Solana CLI `~2.1`, Anchor `0.32.1`, and Yarn.

```bash
yarn install
anchor build
anchor test
```

Tests run against a local validator. All 30 instructions are covered across three test files.

```bash
anchor deploy --provider.cluster mainnet
```

## License

[MIT](LICENSE)
