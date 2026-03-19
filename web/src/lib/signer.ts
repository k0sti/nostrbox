/**
 * Unified signing adapter.
 *
 * When logged in via email (nsec in sessionStorage), signs events directly.
 * When logged in via NIP-07/Amber/Nostr Connect, delegates to those signers.
 */

import { getStoredNsec, signWithNsec, nsecToPubkey } from "./nip49";
import type { EventTemplate, VerifiedEvent } from "nostr-tools/pure";

export type LoginMethod = "extension" | "amber" | "nostr-connect" | "email";

let activeLoginMethod: LoginMethod | null = null;

export function setLoginMethod(method: LoginMethod | null): void {
  activeLoginMethod = method;
}

export function getLoginMethod(): LoginMethod | null {
  return activeLoginMethod;
}

/** Check if current session uses email login (nsec-based signing). */
export function isEmailLogin(): boolean {
  return activeLoginMethod === "email" && getStoredNsec() !== null;
}

/**
 * Sign a Nostr event, using the appropriate method.
 *
 * - Email login: sign with nsec from sessionStorage
 * - NIP-07/Amber: delegate to window.nostr.signEvent
 * - Nostr Connect: delegate to the active NIP-46 signer
 */
export async function signEvent(template: EventTemplate): Promise<VerifiedEvent> {
  if (activeLoginMethod === "email") {
    const nsec = getStoredNsec();
    if (!nsec) throw new Error("No nsec in session — please log in again");
    return signWithNsec(template, nsec);
  }

  // NIP-07 / Amber
  if (activeLoginMethod === "extension" || activeLoginMethod === "amber") {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const ext = (window as any).nostr;
    if (!ext) throw new Error("No NIP-07 extension available");
    return ext.signEvent(template);
  }

  // Nostr Connect — handled by applesauce-signers externally
  throw new Error("Signing not available for current login method");
}

/** Get the current pubkey based on login method. */
export async function getSignerPubkey(): Promise<string | null> {
  if (activeLoginMethod === "email") {
    const nsec = getStoredNsec();
    if (!nsec) return null;
    return nsecToPubkey(nsec);
  }

  if (activeLoginMethod === "extension" || activeLoginMethod === "amber") {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const ext = (window as any).nostr;
    if (!ext) return null;
    return ext.getPublicKey();
  }

  return null;
}
