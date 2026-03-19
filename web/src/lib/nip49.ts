/**
 * NIP-49 ncryptsec encryption/decryption and Nostr key generation.
 *
 * Wraps nostr-tools for client-side key management in the email login flow.
 */

import { encrypt, decrypt } from "nostr-tools/nip49";
import { generateSecretKey, getPublicKey, finalizeEvent } from "nostr-tools/pure";
import { bytesToHex, hexToBytes } from "nostr-tools/utils";
import type { EventTemplate, VerifiedEvent } from "nostr-tools/pure";

export interface KeyPair {
  /** Secret key as hex string */
  nsec: string;
  /** Secret key as raw bytes */
  secretKey: Uint8Array;
  /** Public key as hex string */
  pubkey: string;
}

/** Generate a new Nostr keypair. */
export function generateKeypair(): KeyPair {
  const secretKey = generateSecretKey();
  const pubkey = getPublicKey(secretKey);
  return { nsec: bytesToHex(secretKey), secretKey, pubkey };
}

/**
 * Encrypt an nsec (hex) to ncryptsec using a password.
 * Uses scrypt with logn=16 (good balance of security and speed in browser).
 */
export function encryptNsec(nsecHex: string, password: string): string {
  const secretKey = hexToBytes(nsecHex);
  return encrypt(secretKey, password, 16);
}

/**
 * Decrypt an ncryptsec to nsec (hex) using a password.
 * Throws if password is wrong or data is corrupt.
 */
export function decryptNcryptsec(ncryptsec: string, password: string): string {
  const secretKey = decrypt(ncryptsec, password);
  return bytesToHex(secretKey);
}

/** Sign a Nostr event template with a secret key (hex). */
export function signWithNsec(template: EventTemplate, nsecHex: string): VerifiedEvent {
  const secretKey = hexToBytes(nsecHex);
  return finalizeEvent(template, secretKey);
}

/** Get pubkey from nsec hex. */
export function nsecToPubkey(nsecHex: string): string {
  const secretKey = hexToBytes(nsecHex);
  return getPublicKey(secretKey);
}

/** Store nsec in sessionStorage (cleared on tab close). */
export function storeNsec(nsecHex: string): void {
  sessionStorage.setItem("nostrbox_nsec", nsecHex);
}

/** Retrieve nsec from sessionStorage. */
export function getStoredNsec(): string | null {
  return sessionStorage.getItem("nostrbox_nsec");
}

/** Clear nsec from sessionStorage. */
export function clearStoredNsec(): void {
  sessionStorage.removeItem("nostrbox_nsec");
}
