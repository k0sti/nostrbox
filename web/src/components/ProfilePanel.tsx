import { useEffect, useState } from "react";
import type { NostrIdentity } from "../lib/nostr";
import { compressNpub } from "../lib/nostr";
import { ops, type Actor } from "../lib/api";

interface ProfilePanelProps {
  identity: NostrIdentity;
  onClose: () => void;
  onLogout: () => void;
}

export function ProfilePanel({ identity, onClose, onLogout }: ProfilePanelProps) {
  const [agentActors, setAgentActors] = useState<Actor[]>([]);

  useEffect(() => {
    // Fetch all actors and filter for agents
    ops.actorList().then((res) => {
      if (res.ok && res.data) {
        setAgentActors(res.data.filter((a) => a.kind === "agent"));
      }
    });
  }, []);

  const handleCopyNpub = () => {
    navigator.clipboard.writeText(identity.npub);
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="profile-panel">
          <div className="profile-avatar-large">
            {identity.picture ? (
              <img src={identity.picture} alt="" />
            ) : (
              "👤"
            )}
          </div>
          <div className="profile-name">
            {identity.displayName || "Anonymous"}
          </div>
          <div className="npub-row">
            <span>{compressNpub(identity.npub)}</span>
            <button className="copy-btn" onClick={handleCopyNpub}>
              Copy
            </button>
          </div>

          <div className="card" style={{ textAlign: "left", marginTop: 16 }}>
            <div style={{ fontSize: 13, color: "var(--text-muted)", marginBottom: 8 }}>
              Agent Keys
            </div>
            {agentActors.length === 0 ? (
              <div style={{ fontSize: 13, color: "var(--text-muted)", fontStyle: "italic" }}>
                No agent keys configured yet.
              </div>
            ) : (
              <ul style={{ listStyle: "none", padding: 0, margin: 0 }}>
                {agentActors.map((a) => (
                  <li
                    key={a.pubkey}
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      padding: "4px 0",
                      fontSize: 13,
                    }}
                  >
                    <span style={{ wordBreak: "break-all" }}>
                      {a.npub ? compressNpub(a.npub) : `${a.pubkey.slice(0, 8)}...`}
                    </span>
                    <span className={`badge badge-${a.status}`}>{a.status}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>

          <div style={{ display: "flex", gap: 8, marginTop: 16 }}>
            <button className="modal-close" style={{ flex: 1 }} onClick={onLogout}>
              Logout
            </button>
            <button className="modal-close" style={{ flex: 1 }} onClick={onClose}>
              Close
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
