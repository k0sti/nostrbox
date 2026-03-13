import { useState } from "react";
import { nip19 } from "nostr-tools";
import { ops, type Registration } from "../lib/api";

interface RegisterViewProps {
  onLoginClick: () => void;
}

/** Accept hex pubkey or npub, return hex. */
function resolveToHex(input: string): string {
  const trimmed = input.trim();
  if (trimmed.startsWith("npub1")) {
    try {
      const { type, data } = nip19.decode(trimmed);
      if (type === "npub") return data as string;
    } catch {
      // fall through
    }
  }
  return trimmed;
}

export function RegisterView({ onLoginClick }: RegisterViewProps) {
  const [pubkey, setPubkey] = useState("");
  const [message, setMessage] = useState("");
  const [result, setResult] = useState<Registration | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    const hexPubkey = resolveToHex(pubkey);
    if (!/^[0-9a-f]{64}$/i.test(hexPubkey)) {
      setError("Invalid public key. Enter a 64-char hex key or an npub.");
      setSubmitting(false);
      return;
    }
    try {
      const res = await ops.registrationSubmit(hexPubkey, message.trim() || undefined);
      if (res.ok && res.data) {
        setResult(res.data);
      } else {
        setError(res.error || "Registration failed");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error");
    }
    setSubmitting(false);
  };

  if (result) {
    return (
      <div>
        <h1>Registration Submitted</h1>
        <div className="card">
          <p style={{ marginBottom: 12 }}>
            Your registration request has been submitted. An admin will review it.
          </p>
          <div className="detail-field">
            <span className="detail-label">Status</span>
            <span className={`badge badge-${result.status}`}>{result.status}</span>
          </div>
          <div className="detail-field">
            <span className="detail-label">Pubkey</span>
            <span className="pubkey-short">{result.pubkey.slice(0, 16)}...</span>
          </div>
          <p style={{ marginTop: 16, fontSize: 13, color: "var(--text-muted)" }}>
            Once approved, <button
              onClick={onLoginClick}
              style={{ background: "none", color: "var(--accent)", padding: 0, textDecoration: "underline", fontSize: 13 }}
            >log in</button> to access the full dashboard.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div>
      <h1>Register</h1>
      <div className="card" style={{ maxWidth: 480 }}>
        <p style={{ color: "var(--text-muted)", marginBottom: 16, fontSize: 14 }}>
          Submit a registration request to join this Nostrbox community.
        </p>
        <form onSubmit={handleSubmit}>
          <div className="form-field" style={{ marginBottom: 12 }}>
            <label>Public Key</label>
            <input
              value={pubkey}
              onChange={(e) => setPubkey(e.target.value)}
              placeholder="npub1... or hex pubkey"
              required
            />
          </div>
          <div className="form-field" style={{ marginBottom: 12 }}>
            <label>Message (optional)</label>
            <input
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder="Why do you want to join?"
            />
          </div>
          {error && (
            <p style={{ color: "var(--danger)", fontSize: 13, marginBottom: 8 }}>{error}</p>
          )}
          <button type="submit" className="btn-action" disabled={submitting}>
            {submitting ? "Submitting..." : "Submit Registration"}
          </button>
        </form>
      </div>
    </div>
  );
}
