import { useEffect, useState } from "react";
import { pubkeyToNpub, compressNpub } from "../lib/nostr";

/** Simple in-memory cache so we don't re-fetch the same pubkey in one session. */
const profileCache = new Map<string, { name?: string; picture?: string }>();
const pendingFetches = new Map<string, Promise<{ name?: string; picture?: string }>>();

const PROFILE_RELAYS = [
  "wss://purplepag.es",
  "wss://relay.damus.io",
];

async function fetchProfileMeta(pubkey: string): Promise<{ name?: string; picture?: string }> {
  if (profileCache.has(pubkey)) return profileCache.get(pubkey)!;
  if (pendingFetches.has(pubkey)) return pendingFetches.get(pubkey)!;

  const promise = (async () => {
    for (const url of PROFILE_RELAYS) {
      try {
        const { Relay } = await import("nostr-tools/relay");
        const relay = await Relay.connect(url);
        const result = await new Promise<{ name?: string; picture?: string }>((resolve) => {
          const timer = setTimeout(() => { relay.close(); resolve({}); }, 3000);
          relay.subscribe(
            [{ kinds: [0], authors: [pubkey], limit: 1 }],
            {
              onevent: (event) => {
                clearTimeout(timer);
                try {
                  const meta = JSON.parse(event.content);
                  resolve({
                    name: meta.display_name || meta.name || undefined,
                    picture: meta.picture || undefined,
                  });
                } catch { resolve({}); }
                relay.close();
              },
              oneose: () => { clearTimeout(timer); relay.close(); resolve({}); },
            }
          );
        });
        if (result.name || result.picture) {
          profileCache.set(pubkey, result);
          pendingFetches.delete(pubkey);
          return result;
        }
      } catch { /* try next relay */ }
    }
    const empty = {};
    profileCache.set(pubkey, empty);
    pendingFetches.delete(pubkey);
    return empty;
  })();

  pendingFetches.set(pubkey, promise);
  return promise;
}

interface PubkeyCellProps {
  pubkey: string;
  npub?: string;
  displayName?: string | null;
}

/** Renders a profile icon + name + truncated npub + copy button. */
export function PubkeyCell({ pubkey, npub: npubProp, displayName: nameProp }: PubkeyCellProps) {
  const [profile, setProfile] = useState<{ name?: string; picture?: string }>(
    profileCache.get(pubkey) || {}
  );
  const [copied, setCopied] = useState(false);

  const npub = npubProp || pubkeyToNpub(pubkey);
  const name = nameProp || profile.name;

  useEffect(() => {
    if (!profile.name && !profile.picture) {
      fetchProfileMeta(pubkey).then(setProfile);
    }
  }, [pubkey, profile.name, profile.picture]);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(npub);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="pubkey-cell">
      <div className="pubkey-cell-avatar">
        {profile.picture ? (
          <img src={profile.picture} alt="" />
        ) : (
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
            <circle cx="12" cy="7" r="4" />
          </svg>
        )}
      </div>
      <div className="pubkey-cell-info">
        {name && <span className="pubkey-cell-name">{name}</span>}
        <span className="pubkey-cell-npub">{compressNpub(npub)}</span>
      </div>
      <button className="pubkey-cell-copy" onClick={handleCopy} title="Copy npub">
        {copied ? (
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="20 6 9 17 4 12" />
          </svg>
        ) : (
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
          </svg>
        )}
      </button>
    </div>
  );
}
