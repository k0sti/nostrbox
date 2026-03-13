/**
 * Nostr identity helpers using nostr-tools for bech32/key encoding
 * and applesauce-signers for NIP-46 Nostr Connect.
 */

import { nip19 } from "nostr-tools";
import { Relay } from "nostr-tools/relay";
import { NostrConnectSigner } from "applesauce-signers";
import type { NostrPool } from "applesauce-signers";
import { Observable } from "rxjs";

export interface NostrIdentity {
  pubkey: string;
  npub: string;
  displayName?: string;
  picture?: string;
  /** Actor role from backend — undefined means not yet checked or not registered. */
  role?: "guest" | "member" | "admin" | "owner";
}

/** Convert hex pubkey to bech32 npub. */
export function pubkeyToNpub(hex: string): string {
  try {
    return nip19.npubEncode(hex);
  } catch {
    return "";
  }
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

/** Check if Amber (android signer) is available via NIP-07 interface. */
export function hasAmber(): boolean {
  if (typeof window === "undefined") return false;
  const ua = navigator.userAgent.toLowerCase();
  return "nostr" in window && ua.includes("android");
}

/**
 * Login with NIP-07 web extension (nos2x, Alby, etc.)
 */
export async function loginWithExtension(): Promise<NostrIdentity | null> {
  if (!hasWebExtension()) return null;
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const pubkey: string = await (window as any).nostr.getPublicKey();
    const npub = pubkeyToNpub(pubkey);
    return { pubkey, npub, displayName: undefined, picture: undefined };
  } catch {
    return null;
  }
}

/**
 * Login with Amber (NIP-07 on Android).
 * Amber provides the same window.nostr interface.
 */
export async function loginWithAmber(): Promise<NostrIdentity | null> {
  return loginWithExtension();
}

/**
 * Build a simple NostrPool from nostr-tools relays for applesauce-signers.
 */
function buildPool(_defaultRelays: string[]): NostrPool {
  const relays = new Map<string, Relay>();

  const getRelay = async (url: string): Promise<Relay> => {
    let relay = relays.get(url);
    if (relay) return relay;
    relay = await Relay.connect(url);
    relays.set(url, relay);
    return relay;
  };

  return {
    subscription: (urls, filters) => {
      return new Observable((subscriber) => {
        const subs: Array<{ close: () => void }> = [];

        Promise.all(
          urls.map(async (url) => {
            try {
              const relay = await getRelay(url);
              const sub = relay.subscribe(filters, {
                onevent: (event) => subscriber.next(event),
                oneose: () => {},
              });
              subs.push(sub);
            } catch (e) {
              console.warn(`Failed to subscribe to ${url}:`, e);
            }
          })
        );

        return () => {
          subs.forEach((s) => s.close());
        };
      });
    },
    publish: async (urls, event) => {
      await Promise.allSettled(
        urls.map(async (url) => {
          const relay = await getRelay(url);
          await relay.publish(event);
        })
      );
    },
  };
}

// Track the active NIP-46 signer for cleanup
let activeConnectSigner: NostrConnectSigner | null = null;

/**
 * Login with Nostr Connect (NIP-46).
 * Accepts a bunker:// URI.
 */
export async function loginWithNostrConnect(
  bunkerUri: string
): Promise<NostrIdentity | null> {
  try {
    const { remote, relays, secret } =
      NostrConnectSigner.parseBunkerURI(bunkerUri);

    const pool = buildPool(relays);
    const signer = new NostrConnectSigner({
      relays,
      remote,
      pool,
    });

    await signer.connect(secret);
    activeConnectSigner = signer;

    const pubkey = await signer.getPublicKey();
    const npub = pubkeyToNpub(pubkey);

    return { pubkey, npub, displayName: undefined, picture: undefined };
  } catch (e) {
    console.error("Nostr Connect login failed:", e);
    return null;
  }
}

/** Close the active NIP-46 signer connection. */
export async function disconnectNostrConnect(): Promise<void> {
  if (activeConnectSigner) {
    await activeConnectSigner.close();
    activeConnectSigner = null;
  }
}

/** Well-known public relays for kind-0 profile lookups. */
const PROFILE_RELAYS = [
  "wss://purplepag.es",
  "wss://relay.damus.io",
  "wss://nos.lol",
];

/**
 * Try to fetch kind-0 from a single relay. Returns null on failure.
 */
function fetchKind0FromRelay(
  pubkey: string,
  relayUrl: string,
  timeoutMs = 4000
): Promise<{ displayName?: string; picture?: string } | null> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve(null), timeoutMs);

    Relay.connect(relayUrl)
      .then((relay) => {
        relay.subscribe(
          [{ kinds: [0], authors: [pubkey], limit: 1 }],
          {
            onevent: (event) => {
              clearTimeout(timer);
              try {
                const meta = JSON.parse(event.content);
                resolve({
                  displayName: meta.display_name || meta.name || undefined,
                  picture: meta.picture || undefined,
                });
              } catch {
                resolve(null);
              }
              relay.close();
            },
            oneose: () => {
              clearTimeout(timer);
              relay.close();
              resolve(null);
            },
          }
        );
      })
      .catch(() => {
        clearTimeout(timer);
        resolve(null);
      });
  });
}

/**
 * Fetch kind-0 profile metadata for a pubkey.
 * Tries the server's relay first, then falls back to public relays.
 */
/** Check if a relay URL is a localhost address (unreachable from remote browser). */
function isLocalUrl(url: string): boolean {
  try {
    const u = new URL(url);
    return u.hostname === "localhost" || u.hostname === "127.0.0.1" || u.hostname === "0.0.0.0";
  } catch {
    return false;
  }
}

export async function fetchProfile(
  identity: NostrIdentity,
  serverRelayUrl?: string
): Promise<NostrIdentity> {
  const relaysToTry = [
    ...(serverRelayUrl && !isLocalUrl(serverRelayUrl) ? [serverRelayUrl] : []),
    ...PROFILE_RELAYS,
  ];

  for (const url of relaysToTry) {
    const result = await fetchKind0FromRelay(identity.pubkey, url);
    if (result && (result.displayName || result.picture)) {
      return {
        ...identity,
        displayName: result.displayName || identity.displayName,
        picture: result.picture || identity.picture,
      };
    }
  }

  return identity;
}
