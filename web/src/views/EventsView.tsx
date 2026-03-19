import { useEffect, useRef, useState, useCallback } from "react";
import type { SubscriptionResponse } from "applesauce-relay/types";
import { Subscription } from "rxjs";
import { useRelay } from "../lib/relay-context";

/** Static lookup: kind → { description, nip } */
const KIND_META: Record<number, { desc: string; nip: number }> = {
  0: { desc: "Metadata", nip: 1 },
  1: { desc: "Short Text Note", nip: 1 },
  2: { desc: "Recommend Relay", nip: 1 },
  3: { desc: "Contacts", nip: 2 },
  4: { desc: "Encrypted DM (legacy)", nip: 4 },
  5: { desc: "Event Deletion", nip: 9 },
  6: { desc: "Repost", nip: 18 },
  7: { desc: "Reaction", nip: 25 },
  8: { desc: "Badge Award", nip: 58 },
  9: { desc: "Group Chat Message", nip: 29 },
  10: { desc: "Group Chat (threaded)", nip: 29 },
  11: { desc: "Group Thread", nip: 29 },
  12: { desc: "Group Note", nip: 29 },
  13: { desc: "Seal", nip: 59 },
  14: { desc: "Direct Message", nip: 17 },
  16: { desc: "Generic Repost", nip: 18 },
  40: { desc: "Channel Creation", nip: 28 },
  41: { desc: "Channel Metadata", nip: 28 },
  42: { desc: "Channel Message", nip: 28 },
  43: { desc: "Channel Hide Message", nip: 28 },
  44: { desc: "Channel Mute User", nip: 28 },
  1059: { desc: "Gift Wrap", nip: 59 },
  1063: { desc: "File Metadata", nip: 94 },
  1984: { desc: "Reporting", nip: 56 },
  9000: { desc: "Group Admin Request", nip: 29 },
  9001: { desc: "Group Admin Remove", nip: 29 },
  9002: { desc: "Group Admin Edit", nip: 29 },
  9005: { desc: "Group Admin Delete Event", nip: 29 },
  9006: { desc: "Group Admin Create Invite", nip: 29 },
  9021: { desc: "Group Join Request", nip: 29 },
  9022: { desc: "Group Leave Request", nip: 29 },
  10000: { desc: "Mute List", nip: 51 },
  10001: { desc: "Pin List", nip: 51 },
  10002: { desc: "Relay List Metadata", nip: 65 },
  10009: { desc: "User Groups", nip: 29 },
  10015: { desc: "Interests List", nip: 51 },
  10030: { desc: "User Emoji List", nip: 30 },
  13194: { desc: "Wallet Info", nip: 47 },
  22242: { desc: "Client Authentication", nip: 42 },
  23194: { desc: "Wallet Request", nip: 47 },
  23195: { desc: "Wallet Response", nip: 47 },
  24133: { desc: "Nostr Connect", nip: 46 },
  27235: { desc: "HTTP Auth", nip: 98 },
  30000: { desc: "Follow Sets", nip: 51 },
  30001: { desc: "Generic Lists", nip: 51 },
  30008: { desc: "Profile Badges", nip: 58 },
  30009: { desc: "Badge Definition", nip: 58 },
  30017: { desc: "Stall", nip: 15 },
  30018: { desc: "Product", nip: 15 },
  30023: { desc: "Long-form Content", nip: 23 },
  30024: { desc: "Draft Long-form", nip: 23 },
  30078: { desc: "Application Data", nip: 78 },
  30311: { desc: "Live Event", nip: 53 },
  31922: { desc: "Date-based Calendar Event", nip: 52 },
  31923: { desc: "Time-based Calendar Event", nip: 52 },
  31924: { desc: "Calendar", nip: 52 },
  31925: { desc: "Calendar RSVP", nip: 52 },
  39000: { desc: "Group Metadata", nip: 29 },
  39001: { desc: "Group Admins", nip: 29 },
  39002: { desc: "Group Members", nip: 29 },
};

function getKindMeta(kind: number): { desc: string; nip: number | null } {
  const meta = KIND_META[kind];
  if (meta) return { desc: meta.desc, nip: meta.nip };
  return { desc: `Unknown (kind ${kind})`, nip: null };
}

function nipUrl(nip: number): string {
  return `https://nips.nostr.com/${nip}`;
}

interface KindRow {
  kind: number;
  count: number;
  desc: string;
  nip: number | null;
}

type SortKey = "kind" | "count" | "desc";
type SortDir = "asc" | "desc";

export function EventsView() {
  const { relay, status: relayStatus } = useRelay();
  const [kinds, setKinds] = useState<Map<number, number>>(new Map());
  const [phase, setPhase] = useState<"loading" | "live">("loading");
  const [totalEvents, setTotalEvents] = useState(0);
  const [sortKey, setSortKey] = useState<SortKey>("kind");
  const [sortDir, setSortDir] = useState<SortDir>("asc");

  const rxSubRef = useRef<Subscription | null>(null);
  const kindsRef = useRef<Map<number, number>>(new Map());
  const totalRef = useRef(0);
  const flushTimer = useRef<ReturnType<typeof setInterval> | null>(null);

  const flushState = useCallback(() => {
    setKinds(new Map(kindsRef.current));
    setTotalEvents(totalRef.current);
  }, []);

  const cleanup = useCallback(() => {
    rxSubRef.current?.unsubscribe();
    rxSubRef.current = null;
    if (flushTimer.current) {
      clearInterval(flushTimer.current);
      flushTimer.current = null;
    }
  }, []);

  const startSubscription = useCallback(() => {
    if (!relay) return;
    cleanup();
    kindsRef.current = new Map();
    totalRef.current = 0;
    setKinds(new Map());
    setTotalEvents(0);
    setPhase("loading");

    // Use the shared relay instance (auth handled by RelayProvider)
    const sub$ = relay.subscription([{}]);

    const rxSub = sub$.subscribe({
      next: (msg: SubscriptionResponse) => {
        if (msg === "EOSE") {
          setPhase("live");
          flushState();
          // Switch to slower flush interval after EOSE
          if (flushTimer.current) clearInterval(flushTimer.current);
          flushTimer.current = setInterval(flushState, 2000);
          return;
        }
        // It's a NostrEvent
        const event = msg;
        const prev = kindsRef.current.get(event.kind) ?? 0;
        kindsRef.current.set(event.kind, prev + 1);
        totalRef.current++;
      },
      error: (e) => {
        console.error("Relay subscription error:", e);
        setPhase("live"); // Stop loading indicator on error
      },
    });

    rxSubRef.current = rxSub;

    // Flush UI every 500ms during loading to show progress
    flushTimer.current = setInterval(flushState, 500);
  }, [relay, cleanup, flushState]);

  const handleReset = useCallback(() => {
    startSubscription();
  }, [startSubscription]);

  // Re-subscribe when relay becomes available or status changes to authenticated
  useEffect(() => {
    if (relay && (relayStatus === "authenticated" || relayStatus === "connected")) {
      startSubscription();
    }
    return cleanup;
  }, [relay, relayStatus, startSubscription, cleanup]);

  // Build sorted rows
  const rows: KindRow[] = Array.from(kinds.entries())
    .map(([kind, count]) => {
      const meta = getKindMeta(kind);
      return { kind, count, desc: meta.desc, nip: meta.nip };
    })
    .sort((a, b) => {
      let cmp = 0;
      if (sortKey === "kind") cmp = a.kind - b.kind;
      else if (sortKey === "count") cmp = a.count - b.count;
      else cmp = a.desc.localeCompare(b.desc);
      return sortDir === "asc" ? cmp : -cmp;
    });

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortDir(key === "count" ? "desc" : "asc");
    }
  };

  const sortIndicator = (key: SortKey) => {
    if (sortKey !== key) return "";
    return sortDir === "asc" ? " ▲" : " ▼";
  };

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 16 }}>
        <h1 style={{ margin: 0 }}>Events</h1>
        <button className="btn-action" onClick={handleReset}>
          ↻ Reset
        </button>
      </div>

      <div className="stat-grid" style={{ marginBottom: 20 }}>
        <div className="stat-card">
          <div className="stat-value">{totalEvents.toLocaleString()}</div>
          <div className="stat-label">Total Events</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">{kinds.size}</div>
          <div className="stat-label">Unique Kinds</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">
            {phase === "loading" ? (
              <span style={{ color: "var(--text-muted)", fontSize: 18 }}>● Collecting…</span>
            ) : (
              <span style={{ color: "var(--success)", fontSize: 18 }}>● Live</span>
            )}
          </div>
          <div className="stat-label">Status</div>
        </div>
      </div>

      {rows.length === 0 && phase === "loading" ? (
        <div className="card" style={{ textAlign: "center", color: "var(--text-muted)" }}>
          Connecting to relay and collecting events…
        </div>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th
                onClick={() => handleSort("kind")}
                style={{ cursor: "pointer", userSelect: "none" }}
              >
                Kind{sortIndicator("kind")}
              </th>
              <th
                onClick={() => handleSort("count")}
                style={{ cursor: "pointer", userSelect: "none", textAlign: "right" }}
              >
                Count{sortIndicator("count")}
              </th>
              <th
                onClick={() => handleSort("desc")}
                style={{ cursor: "pointer", userSelect: "none" }}
              >
                Description{sortIndicator("desc")}
              </th>
              <th>Spec</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.kind}>
                <td>
                  <code style={{ fontSize: 13 }}>{row.kind}</code>
                </td>
                <td style={{ textAlign: "right", fontVariantNumeric: "tabular-nums" }}>
                  {row.count.toLocaleString()}
                </td>
                <td>{row.desc}</td>
                <td>
                  {row.nip != null ? (
                    <a
                      href={nipUrl(row.nip)}
                      target="_blank"
                      rel="noopener noreferrer"
                      style={{ color: "var(--accent)", textDecoration: "none", fontSize: 13 }}
                    >
                      NIP-{String(row.nip).padStart(2, "0")}
                    </a>
                  ) : (
                    <span style={{ color: "var(--text-muted)" }}>—</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
