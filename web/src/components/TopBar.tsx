import { compressNpub, type NostrIdentity } from "../lib/nostr";
import { useRelay, type RelayStatus } from "../lib/relay-context";

const STATUS_CONFIG: Record<RelayStatus, { label: string; dot: string; className: string; title: string }> = {
  disconnected: { label: "Offline", dot: "○", className: "relay-badge-off", title: "Relay disconnected" },
  connecting: { label: "Connecting", dot: "◌", className: "relay-badge-warn", title: "Connecting to relay…" },
  connected: { label: "No Auth", dot: "●", className: "relay-badge-warn", title: "Connected but not authenticated — NIP-07 extension required" },
  authenticated: { label: "Relay", dot: "●", className: "relay-badge-ok", title: "Authenticated to relay" },
  "auth-failed": { label: "Auth Failed", dot: "✕", className: "relay-badge-err", title: "Authentication failed — check NIP-07 extension" },
  error: { label: "Error", dot: "✕", className: "relay-badge-err", title: "Relay connection error" },
};

interface TopBarProps {
  identity: NostrIdentity | null;
  onLoginClick: () => void;
  onProfileClick: () => void;
  onSettingsClick: () => void;
  onMenuClick: () => void;
}

export function TopBar({
  identity,
  onLoginClick,
  onProfileClick,
  onSettingsClick,
  onMenuClick,
}: TopBarProps) {
  const { status } = useRelay();
  const cfg = STATUS_CONFIG[status];

  return (
    <header className="topbar">
      <div className="topbar-left">
        <button className="hamburger-btn" onClick={onMenuClick}>
          &#9776;
        </button>
        {identity ? (
          <>
            <button className="avatar-btn" onClick={onProfileClick} title="Profile">
              {identity.picture ? (
                <img src={identity.picture} alt="" />
              ) : (
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                  <circle cx="12" cy="7" r="4" />
                </svg>
              )}
            </button>
            <span style={{ display: "flex", flexDirection: "column", justifyContent: "center", fontSize: 13, color: "var(--text)", marginLeft: 4, minWidth: 0, lineHeight: 1.2 }}>
              <span style={{ fontWeight: 500, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis", maxWidth: 120 }}>
                {identity.displayName || "Anonymous"}
              </span>
              <span style={{ color: "var(--text-muted)", fontFamily: "monospace", fontSize: 11, whiteSpace: "nowrap" }}>
                {compressNpub(identity.npub)}
              </span>
            </span>
          </>
        ) : (
          <button className="login-btn" onClick={onLoginClick}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" />
              <polyline points="10 17 15 12 10 7" />
              <line x1="15" y1="12" x2="3" y2="12" />
            </svg>
            Login
          </button>
        )}
      </div>
      <div className="topbar-center">
        Nostrbox
        <span className={`relay-badge ${cfg.className}`} title={cfg.title}>
          <span className="relay-badge-dot">{cfg.dot}</span>{" "}
          <span className="relay-badge-label">{cfg.label}</span>
        </span>
      </div>
      <div className="topbar-right">
        <button className="settings-btn" onClick={onSettingsClick}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
          </svg>
        </button>
      </div>
    </header>
  );
}
