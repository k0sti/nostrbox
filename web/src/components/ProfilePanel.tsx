import type { NostrIdentity } from "../lib/nostr";
import { compressNpub } from "../lib/nostr";

interface ProfilePanelProps {
  identity: NostrIdentity;
  onClose: () => void;
  onLogout: () => void;
}

export function ProfilePanel({ identity, onClose, onLogout }: ProfilePanelProps) {
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

          {/* TODO: Agent npub list */}
          <div className="card" style={{ textAlign: "left", marginTop: 16 }}>
            <div style={{ fontSize: 13, color: "var(--text-muted)", marginBottom: 8 }}>
              Agent Keys
            </div>
            <div style={{ fontSize: 13, color: "var(--text-muted)", fontStyle: "italic" }}>
              No agent keys configured yet.
            </div>
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
