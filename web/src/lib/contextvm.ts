/**
 * ContextVM client using @contextvm/sdk for Nostr-native transport.
 *
 * Falls back to HTTP POST /api/op when no signer is available.
 */

import { NostrClientTransport, PrivateKeySigner } from "@contextvm/sdk";
import type { NostrSigner } from "@contextvm/sdk";
import type { OperationResponse } from "./api";
import { loadSettings } from "./settings";

const API_BASE = import.meta.env.VITE_API_URL ?? "";

let transport: NostrClientTransport | null = null;
let pendingRequests: Map<
  number,
  { resolve: (v: OperationResponse) => void; reject: (e: Error) => void }
> = new Map();
let requestId = 0;

/** NIP-07 signer wrapper for @contextvm/sdk. */
class Nip07Signer implements NostrSigner {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private ext: any;

  constructor() {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    this.ext = (window as any).nostr;
  }

  async getPublicKey(): Promise<string> {
    return this.ext.getPublicKey();
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  async signEvent(event: any): Promise<any> {
    return this.ext.signEvent(event);
  }

  get nip44() {
    if (!this.ext.nip44) return undefined;
    return {
      encrypt: (pubkey: string, plaintext: string) =>
        this.ext.nip44.encrypt(pubkey, plaintext),
      decrypt: (pubkey: string, ciphertext: string) =>
        this.ext.nip44.decrypt(pubkey, ciphertext),
    };
  }
}

interface ServerInfo {
  relay_url: string;
  pubkey: string;
}

/** Fetch server info (relay URL + pubkey) from the API. */
async function fetchServerInfo(): Promise<ServerInfo | null> {
  try {
    const res = await fetch(`${API_BASE}/api/relay-info`, {
      headers: { Accept: "application/json" },
    });
    const data = await res.json();
    if (data.relay_url && data.pubkey) {
      return { relay_url: data.relay_url, pubkey: data.pubkey };
    }
    return null;
  } catch {
    return null;
  }
}

/**
 * Connect the ContextVM Nostr transport.
 * Uses NIP-07 signer if available, otherwise generates an ephemeral key.
 */
/** Last connection error, if any. */
let lastConnectError: string | null = null;

export function getLastConnectError(): string | null {
  return lastConnectError;
}

export async function connectTransport(): Promise<boolean> {
  if (transport) return true;
  lastConnectError = null;

  const info = await fetchServerInfo();
  if (!info) {
    lastConnectError = "Could not fetch server info from /api/relay-info";
    console.warn("[nostrbox] CVM connect failed:", lastConnectError);
    return false;
  }
  if (!info.pubkey) {
    lastConnectError = "Server has no identity configured (add identity_nsec to nostrbox.toml)";
    console.warn("[nostrbox] CVM connect failed:", lastConnectError);
    return false;
  }

  const hasExtension =
    typeof window !== "undefined" && "nostr" in window;
  const signer: NostrSigner = hasExtension
    ? new Nip07Signer()
    : new PrivateKeySigner(); // ephemeral key

  // Use custom relay URL from settings if configured, otherwise server default
  const settings = loadSettings();
  const relayUrl = settings.relayUrl || info.relay_url;

  console.info(`[nostrbox] CVM connecting to relay ${relayUrl} (server pubkey: ${info.pubkey.slice(0, 8)}...)`);

  try {
    transport = new NostrClientTransport({
      signer,
      serverPubkey: info.pubkey,
      relayHandler: [relayUrl],
      isStateless: true,
    });
    await transport.start();

    // Listen for responses
    transport.onmessage = (message) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const msg = message as any;
      if (msg.id !== undefined && pendingRequests.has(msg.id)) {
        const pending = pendingRequests.get(msg.id)!;
        pendingRequests.delete(msg.id);
        if (msg.error) {
          pending.resolve({
            ok: false,
            error: msg.error.message,
            error_code: String(msg.error.code),
          });
        } else {
          pending.resolve(msg.result);
        }
      }
    };

    console.info("[nostrbox] CVM transport connected");
    return true;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    lastConnectError = `Transport failed: ${msg}`;
    console.warn("[nostrbox] CVM connect failed:", lastConnectError);
    transport = null;
    return false;
  }
}

/** Disconnect the transport. */
export async function disconnectTransport(): Promise<void> {
  if (transport) {
    await transport.close();
    transport = null;
    pendingRequests.clear();
  }
}

/** Check if the Nostr transport is connected. */
export function isTransportConnected(): boolean {
  return transport !== null;
}

/**
 * Call a ContextVM operation via Nostr transport.
 * Falls back to HTTP if transport is not connected.
 */
export async function callOpNostr<T = unknown>(
  op: string,
  params: Record<string, unknown> = {}
): Promise<OperationResponse<T>> {
  // If transport is connected, use it
  if (transport) {
    const id = ++requestId;
    const promise = new Promise<OperationResponse<T>>((resolve, reject) => {
      pendingRequests.set(id, {
        resolve: resolve as (v: OperationResponse) => void,
        reject,
      });

      // Timeout after 15s
      setTimeout(() => {
        if (pendingRequests.has(id)) {
          pendingRequests.delete(id);
          reject(new Error("ContextVM request timeout"));
        }
      }, 15000);
    });

    transport.send({
      jsonrpc: "2.0",
      id,
      method: op,
      params,
    });

    return promise;
  }

  // Fallback to HTTP
  const res = await fetch(`${API_BASE}/api/op`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ op, params }),
  });
  return res.json();
}
