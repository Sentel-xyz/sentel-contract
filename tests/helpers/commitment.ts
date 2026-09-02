import { createHash } from "crypto";
import { PublicKey } from "@solana/web3.js";

/**
 * Mirror of `commitment::swap_commitment` in the program.
 *
 * A proposal records this hash of the Jupiter payload and the ordered account
 * keys it authorises; execution recomputes it and refuses to run anything else.
 * Keep the two implementations in step.
 */
export function swapCommitment(data: Buffer, accountKeys: PublicKey[]): number[] {
  const hash = createHash("sha256");
  hash.update(data);
  for (const key of accountKeys) {
    hash.update(key.toBuffer());
  }
  return Array.from(hash.digest());
}

/** Commitment for a swap that carries no route, used where tests never reach the CPI. */
export const EMPTY_COMMITMENT: number[] = new Array(32).fill(0);
