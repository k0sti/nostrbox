/**
 * Nostr identity helpers.
 *
 * TODO: Integrate applesauce library for full Nostr web integration.
 * See: https://hzrd149.github.io/applesauce/
 *
 * For now this provides stub types and helpers for the login flow.
 */

export interface NostrIdentity {
  pubkey: string;
  npub: string;
  displayName?: string;
  picture?: string;
}

/** Compress npub for display: npub1xxxx...xxxx (first 10 + last 4) */
export function compressNpub(npub: string): string {
  if (npub.length <= 18) return npub;
  return `${npub.slice(0, 10)}...${npub.slice(-4)}`;
}

/** Check if NIP-07 web extension (window.nostr) is available. */
export function hasWebExtension(): boolean {
  return typeof window !== "undefined" && "nostr" in window;
}

/** Check if Amber (android signer) is available. */
export function hasAmber(): boolean {
  // TODO: Detect Amber availability via intent or bridge
  return false;
}

/**
 * Login with NIP-07 web extension.
 * TODO: Use applesauce signer integration.
 */
export async function loginWithExtension(): Promise<NostrIdentity | null> {
  if (!hasWebExtension()) return null;
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const pubkey: string = await (window as any).nostr.getPublicKey();
    const npub = `npub1${pubkey.slice(0, 20)}`; // TODO: proper bech32 encoding
    return { pubkey, npub, displayName: undefined, picture: undefined };
  } catch {
    return null;
  }
}

/**
 * Login with Nostr Connect (NIP-46).
 * TODO: Implement with applesauce NostrConnect signer.
 */
export async function loginWithNostrConnect(
  _relay: string
): Promise<NostrIdentity | null> {
  // TODO: Implement NIP-46 Nostr Connect flow
  console.warn("Nostr Connect login not yet implemented");
  return null;
}
