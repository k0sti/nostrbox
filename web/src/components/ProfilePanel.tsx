import { useEffect, useState } from "react";
import type { NostrIdentity } from "../lib/nostr";
import { compressNpub } from "../lib/nostr";
import { ops, type Actor } from "../lib/api";
import { isEmailLogin, getLoginMethod } from "../lib/signer";
import { getStoredNsec, encryptNsec } from "../lib/nip49";

interface ProfilePanelProps {
  identity: NostrIdentity;
  onClose: () => void;
  onLogout: () => void;
}

type SubPanel = null | "sovereign" | "change-password";

export function ProfilePanel({ identity, onClose, onLogout }: ProfilePanelProps) {
  const [agentActors, setAgentActors] = useState<Actor[]>([]);
  const [subPanel, setSubPanel] = useState<SubPanel>(null);
  const [showNsec, setShowNsec] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  // Password change
  const [newPassword, setNewPassword] = useState("");
  const [newPasswordConfirm, setNewPasswordConfirm] = useState("");

  const emailLogin = isEmailLogin();
  const loginMethod = getLoginMethod();

  useEffect(() => {
    ops.actorList().then((res) => {
      if (res.ok && res.data) {
        setAgentActors(res.data.filter((a) => a.kind === "agent"));
      }
    });
  }, []);

  const handleCopy = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    setCopied(label);
    setTimeout(() => setCopied(null), 2000);
  };

  const handleGoSovereign = async () => {
    setError(null);
    setSubmitting(true);
    try {
      const res = await ops.emailClear();
      if (res.ok) {
        setSuccess("Server-side keys cleared. You must now use NIP-07 or Amber to log in.");
      } else {
        setError(res.error || "Failed to clear server-side keys");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error");
    }
    setSubmitting(false);
  };

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSuccess(null);

    if (newPassword.length < 8) {
      setError("New password must be at least 8 characters");
      return;
    }
    if (newPassword !== newPasswordConfirm) {
      setError("Passwords do not match");
      return;
    }

    const nsec = getStoredNsec();
    if (!nsec) {
      setError("No active session — please log in again");
      return;
    }

    setSubmitting(true);
    try {
      const newNcryptsec = encryptNsec(nsec, newPassword);
      const res = await ops.emailChangePassword(newNcryptsec);
      if (res.ok) {
        setSuccess("Password changed successfully");
        setNewPassword("");
        setNewPasswordConfirm("");
      } else {
        setError(res.error || "Failed to change password");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to change password");
    }
    setSubmitting(false);
  };

  const nsec = emailLogin ? getStoredNsec() : null;

  // ── Go Sovereign Sub-panel ──
  if (subPanel === "sovereign") {
    return (
      <div className="modal-overlay" onClick={onClose}>
        <div className="modal" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 440 }}>
          <h2>Go Sovereign</h2>
          <p style={{ color: "var(--text-muted)", fontSize: 13, marginBottom: 16 }}>
            Take full control of your keys. After clearing server-side storage, you'll need a NIP-07 extension or Amber to log in.
          </p>

          {nsec && (
            <div className="card" style={{ marginBottom: 12 }}>
              <div style={{ fontSize: 12, color: "var(--text-muted)", textTransform: "uppercase", fontWeight: 600, marginBottom: 4 }}>
                Your Private Key (nsec)
              </div>
              {showNsec ? (
                <div style={{ fontFamily: "monospace", fontSize: 12, color: "var(--danger)", wordBreak: "break-all", marginBottom: 8 }}>
                  {nsec}
                </div>
              ) : (
                <button
                  className="btn-action"
                  onClick={() => setShowNsec(true)}
                  style={{ fontSize: 12, padding: "4px 10px", marginBottom: 8 }}
                >
                  Reveal
                </button>
              )}
              <button
                className="copy-btn"
                onClick={() => handleCopy(nsec, "nsec")}
                style={{ fontSize: 12 }}
              >
                {copied === "nsec" ? "Copied!" : "Copy nsec"}
              </button>
            </div>
          )}

          <div style={{ background: "var(--bg-panel)", border: "1px solid var(--border)", borderRadius: "var(--radius)", padding: "10px 14px", marginBottom: 12, fontSize: 12, color: "var(--warning)" }}>
            Save your nsec before clearing. Without it, you lose access to this identity forever.
          </div>

          {error && <p style={{ color: "var(--danger)", fontSize: 13, marginBottom: 8 }}>{error}</p>}
          {success && <p style={{ color: "var(--success)", fontSize: 13, marginBottom: 8 }}>{success}</p>}

          {!success && (
            <button
              className="btn-action btn-danger"
              style={{ width: "100%", padding: "10px 16px" }}
              onClick={handleGoSovereign}
              disabled={submitting}
            >
              {submitting ? "Clearing..." : "Clear Server-Side Keys"}
            </button>
          )}

          <button className="modal-close" onClick={() => { setSubPanel(null); setError(null); setSuccess(null); setShowNsec(false); }}>
            Back
          </button>
        </div>
      </div>
    );
  }

  // ── Change Password Sub-panel ──
  if (subPanel === "change-password") {
    return (
      <div className="modal-overlay" onClick={onClose}>
        <div className="modal" onClick={(e) => e.stopPropagation()}>
          <h2>Change Password</h2>
          <p style={{ color: "var(--text-muted)", fontSize: 13, marginBottom: 16 }}>
            Re-encrypt your private key with a new password.
          </p>

          <form onSubmit={handleChangePassword}>
            <div className="form-field" style={{ marginBottom: 12 }}>
              <label>New Password</label>
              <input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                placeholder="New password (min 8 characters)"
                required
                minLength={8}
              />
            </div>
            <div className="form-field" style={{ marginBottom: 12 }}>
              <label>Confirm New Password</label>
              <input
                type="password"
                value={newPasswordConfirm}
                onChange={(e) => setNewPasswordConfirm(e.target.value)}
                placeholder="Confirm new password"
                required
              />
            </div>

            {error && <p style={{ color: "var(--danger)", fontSize: 13, marginBottom: 8 }}>{error}</p>}
            {success && <p style={{ color: "var(--success)", fontSize: 13, marginBottom: 8 }}>{success}</p>}

            <button type="submit" className="btn-action" style={{ width: "100%", padding: "10px 16px" }} disabled={submitting}>
              {submitting ? "Changing..." : "Change Password"}
            </button>
          </form>

          <button className="modal-close" onClick={() => { setSubPanel(null); setError(null); setSuccess(null); }}>
            Back
          </button>
        </div>
      </div>
    );
  }

  // ── Main Profile Panel ──
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
            <button className="copy-btn" onClick={() => handleCopy(identity.npub, "npub")}>
              {copied === "npub" ? "Copied!" : "Copy"}
            </button>
          </div>

          {loginMethod && (
            <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 12 }}>
              Signed in via {loginMethod === "email" ? "email" : loginMethod === "extension" ? "extension" : loginMethod === "amber" ? "Amber" : "Nostr Connect"}
            </div>
          )}

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

          {emailLogin && (
            <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
              <button
                className="btn-action"
                style={{ flex: 1, fontSize: 13, padding: "8px 12px" }}
                onClick={() => setSubPanel("change-password")}
              >
                Change Password
              </button>
              <button
                className="btn-action"
                style={{ flex: 1, fontSize: 13, padding: "8px 12px", background: "var(--warning)", color: "#000" }}
                onClick={() => setSubPanel("sovereign")}
              >
                Go Sovereign
              </button>
            </div>
          )}

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
