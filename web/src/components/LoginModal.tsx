import { hasWebExtension, hasAmber } from "../lib/nostr";

interface LoginModalProps {
  onClose: () => void;
  onLoginExtension: () => void;
  onLoginAmber: () => void;
  onLoginNostrConnect: () => void;
}

export function LoginModal({
  onClose,
  onLoginExtension,
  onLoginAmber,
  onLoginNostrConnect,
}: LoginModalProps) {
  const webExtAvailable = hasWebExtension();
  const amberAvailable = hasAmber();

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
          🔑 Login with Web Extension
          {!webExtAvailable && <span style={{ fontSize: 12, marginLeft: 8, color: "var(--text-muted)" }}>(not detected)</span>}
        </button>

        <button
          className="login-option"
          onClick={onLoginAmber}
          disabled={!amberAvailable}
          style={{ opacity: amberAvailable ? 1 : 0.4 }}
        >
          📱 Login with Amber
          {!amberAvailable && <span style={{ fontSize: 12, marginLeft: 8, color: "var(--text-muted)" }}>(not detected)</span>}
        </button>

        <button className="login-option" onClick={onLoginNostrConnect}>
          🔗 Login with Nostr Connect
        </button>

        <button className="modal-close" onClick={onClose}>
          Cancel
        </button>
      </div>
    </div>
  );
}
