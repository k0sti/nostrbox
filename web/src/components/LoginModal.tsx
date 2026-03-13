import { useState } from "react";
import { hasWebExtension, hasAmber } from "../lib/nostr";

interface LoginModalProps {
  onClose: () => void;
  onLoginExtension: () => void;
  onLoginAmber: () => void;
  onLoginNostrConnect: (bunkerUrl: string) => void;
}

export function LoginModal({
  onClose,
  onLoginExtension,
  onLoginAmber,
  onLoginNostrConnect,
}: LoginModalProps) {
  const webExtAvailable = hasWebExtension();
  const amberAvailable = hasAmber();
  const [showBunker, setShowBunker] = useState(false);
  const [bunkerUrl, setBunkerUrl] = useState("");

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>Login</h2>

        <button
          className="login-option"
          onClick={onLoginExtension}
          disabled={!webExtAvailable}
          style={{ opacity: webExtAvailable ? 1 : 0.4 }}
        >
          Login with Web Extension
          {!webExtAvailable && <span style={{ fontSize: 12, marginLeft: 8, color: "var(--text-muted)" }}>(not detected)</span>}
        </button>

        <button
          className="login-option"
          onClick={onLoginAmber}
          disabled={!amberAvailable}
          style={{ opacity: amberAvailable ? 1 : 0.4 }}
        >
          Login with Amber
          {!amberAvailable && <span style={{ fontSize: 12, marginLeft: 8, color: "var(--text-muted)" }}>(not detected)</span>}
        </button>

        {!showBunker ? (
          <button className="login-option" onClick={() => setShowBunker(true)}>
            Login with Nostr Connect
          </button>
        ) : (
          <div style={{ marginTop: 8 }}>
            <input
              type="text"
              placeholder="bunker://... or npub..."
              value={bunkerUrl}
              onChange={(e) => setBunkerUrl(e.target.value)}
              style={{
                width: "100%",
                padding: "8px 12px",
                borderRadius: 8,
                border: "1px solid var(--border)",
                background: "var(--bg-card)",
                color: "var(--text-primary)",
                fontSize: 13,
                boxSizing: "border-box",
              }}
            />
            <button
              className="login-option"
              onClick={() => onLoginNostrConnect(bunkerUrl)}
              disabled={!bunkerUrl.trim()}
              style={{ marginTop: 8, opacity: bunkerUrl.trim() ? 1 : 0.4 }}
            >
              Connect
            </button>
          </div>
        )}

        <button className="modal-close" onClick={onClose}>
          Cancel
        </button>
      </div>
    </div>
  );
}
