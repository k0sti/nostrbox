/**
 * NIP-98 HTTP Auth — signs kind 27235 events for authenticating HTTP requests.
 *
 * The signed event is base64-encoded and sent as `Authorization: Nostr <base64>`.
 * The server verifies the signature, timestamp, URL, and method to extract
 * a cryptographically verified caller pubkey.
 */

import { signEvent, getLoginMethod } from "./signer";
import type { EventTemplate } from "nostr-tools/pure";

/**
 * Create a NIP-98 Authorization header value.
 *
 * Returns `Nostr <base64(JSON(signed_event))>` or null if no signer is active.
 */
export async function createNip98Auth(
  url: string,
  method: string,
): Promise<string | null> {
  if (!getLoginMethod()) return null;

  const template: EventTemplate = {
    kind: 27235,
    created_at: Math.floor(Date.now() / 1000),
    tags: [
      ["u", url],
      ["method", method.toUpperCase()],
    ],
    content: "",
  };

  try {
    const signed = await signEvent(template);
    return `Nostr ${btoa(JSON.stringify(signed))}`;
  } catch (e) {
    console.warn("[nip98] Failed to sign auth event:", e);
    return null;
  }
}
