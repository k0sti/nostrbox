import { useState } from "react";
import { hasWebExtension, hasAmber, pubkeyToNpub } from "../lib/nostr";
import { generateKeypair, encryptNsec, decryptNcryptsec, storeNsec } from "../lib/nip49";
import { ops } from "../lib/api";

interface LoginModalProps {
  onClose: () => void;
  onLoginExtension: () => void;
  onLoginAmber: () => void;
  onLoginNostrConnect: (bunkerUrl: string) => void;
  onEmailLogin: (pubkey: string, npub: string, nsecHex: string) => void;
}

type EmailStep = "initial" | "register" | "login" | "check-email" | "password-decrypt";

export function LoginModal({
  onClose,
  onLoginExtension,
  onLoginAmber,
  onLoginNostrConnect,
  onEmailLogin,
}: LoginModalProps) {
  const webExtAvailable = hasWebExtension();
  const amberAvailable = hasAmber();
  const [showBunker, setShowBunker] = useState(false);
  const [bunkerUrl, setBunkerUrl] = useState("");

  // Email flow state
  const [emailStep, setEmailStep] = useState<EmailStep>("initial");
  const [email, setEmail] = useState(() => localStorage.getItem("nostrbox_email") || "");
  const [password, setPassword] = useState("");
  const [passwordConfirm, setPasswordConfirm] = useState("");
  const [generatedKeys, setGeneratedKeys] = useState<{ nsec: string; pubkey: string; npub: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  // For login flow — stored after redeem
  const [redeemData] = useState<{ npub: string; ncryptsec: string } | null>(null);

  const handleGenerateKeys = () => {
    const kp = generateKeypair();
    const npub = pubkeyToNpub(kp.pubkey);
    setGeneratedKeys({ nsec: kp.nsec, pubkey: kp.pubkey, npub });
  };

  const handleEmailRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!generatedKeys) {
      setError("Generate keys first");
      return;
    }
    if (password.length < 8) {
      setError("Password must be at least 8 characters");
      return;
    }
    if (password !== passwordConfirm) {
      setError("Passwords do not match");
      return;
    }
    if (!email.trim()) {
      setError("Email is required");
      return;
    }

    setSubmitting(true);
    try {
      const ncryptsec = encryptNsec(generatedKeys.nsec, password);
      const res = await ops.emailRegister(generatedKeys.npub, ncryptsec, email.trim().toLowerCase());
      if (res.ok) {
        // Auto-login after registration — store nsec and proceed
        localStorage.setItem("nostrbox_email", email.trim().toLowerCase());
        storeNsec(generatedKeys.nsec);
        onEmailLogin(generatedKeys.pubkey, generatedKeys.npub, generatedKeys.nsec);
      } else {
        setError(res.error || "Registration failed");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error");
    }
    setSubmitting(false);
  };

  const handleEmailLoginSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!email.trim()) {
      setError("Email is required");
      return;
    }
    setSubmitting(true);
    try {
      const res = await ops.emailLogin(email.trim().toLowerCase());
      if (res.ok) {
        localStorage.setItem("nostrbox_email", email.trim().toLowerCase());
        setEmailStep("check-email");
      } else {
        setError(res.error || "Failed to send login email");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error");
    }
    setSubmitting(false);
  };

  const handlePasswordDecrypt = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!redeemData) return;

    try {
      const nsecHex = decryptNcryptsec(redeemData.ncryptsec, password);
      storeNsec(nsecHex);
      // Derive pubkey from the npub returned by server
      const { decodePointer } = await import("applesauce-core/helpers/pointers");
      const { data: pubkey } = decodePointer(redeemData.npub) as { type: string; data: string };
      onEmailLogin(pubkey, redeemData.npub, nsecHex);
    } catch {
      setError("Wrong password. If you forgot it, your keys cannot be recovered.");
    }
  };

  // Expose setRedeemData for magic link flow (called from App.tsx via ref pattern)
  // Instead, App.tsx will render this modal with redeemData pre-set
  // For now, we use the prop-based approach in App.tsx

  // ── Email Registration Form ──
  if (emailStep === "register") {
    return (
      <div className="modal-overlay" onMouseDown={onClose}>
        <div className="modal" onMouseDown={(e) => e.stopPropagation()} style={{ maxWidth: 440 }}>
          <h2>Register with Email</h2>
          <p style={{ color: "var(--text-muted)", fontSize: 13, marginBottom: 16 }}>
            Generate a Nostr keypair, protect it with a password, and register with your email.
          </p>

          {!generatedKeys ? (
            <button className="btn-action" onClick={handleGenerateKeys} style={{ width: "100%", padding: "10px 16px", marginBottom: 12 }}>
              Generate Keys
            </button>
          ) : (
            <div className="card" style={{ marginBottom: 12 }}>
              <div style={{ fontSize: 12, color: "var(--text-muted)", textTransform: "uppercase", fontWeight: 600, marginBottom: 4 }}>
                Your Public Key
              </div>
              <div style={{ fontFamily: "monospace", fontSize: 13, color: "var(--text)", wordBreak: "break-all" }}>
                {generatedKeys.npub}
              </div>
            </div>
          )}

          {generatedKeys && (
            <form onSubmit={handleEmailRegister}>
              <div className="form-field" style={{ marginBottom: 12 }}>
                <label>Password</label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="Encrypts your private key"
                  required
                  minLength={8}
                />
              </div>
              <div className="form-field" style={{ marginBottom: 12 }}>
                <label>Confirm Password</label>
                <input
                  type="password"
                  value={passwordConfirm}
                  onChange={(e) => setPasswordConfirm(e.target.value)}
                  placeholder="Confirm password"
                  required
                />
              </div>
              <div className="form-field" style={{ marginBottom: 12 }}>
                <label>Email</label>
                <input
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  placeholder="your@email.com"
                  required
                />
              </div>

              <div className="email-warning" style={{ background: "var(--bg-panel)", border: "1px solid var(--border)", borderRadius: "var(--radius)", padding: "10px 14px", marginBottom: 12, fontSize: 12, color: "var(--warning)" }}>
                If you forget your password, your keys cannot be recovered. There is no password reset.
              </div>

              {error && <p style={{ color: "var(--danger)", fontSize: 13, marginBottom: 8 }}>{error}</p>}

              <button type="submit" className="btn-action" style={{ width: "100%", padding: "10px 16px" }} disabled={submitting}>
                {submitting ? "Registering..." : "Register"}
              </button>
            </form>
          )}

          <button className="modal-close" onClick={() => { setEmailStep("initial"); setError(null); }}>
            Back
          </button>
        </div>
      </div>
    );
  }

  // ── Email Login Form ──
  if (emailStep === "login") {
    return (
      <div className="modal-overlay" onMouseDown={onClose}>
        <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
          <h2>Login with Email</h2>
          <form onSubmit={handleEmailLoginSubmit}>
            <div className="form-field" style={{ marginBottom: 12 }}>
              <label>Email</label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="your@email.com"
                required
              />
            </div>
            {error && <p style={{ color: "var(--danger)", fontSize: 13, marginBottom: 8 }}>{error}</p>}
            <button type="submit" className="btn-action" style={{ width: "100%", padding: "10px 16px" }} disabled={submitting}>
              {submitting ? "Sending..." : "Send Login Link"}
            </button>
          </form>
          <button className="modal-close" onClick={() => { setEmailStep("initial"); setError(null); }}>
            Back
          </button>
        </div>
      </div>
    );
  }

  // ── Check Your Email ──
  if (emailStep === "check-email") {
    return (
      <div className="modal-overlay" onMouseDown={onClose}>
        <div className="modal" onMouseDown={(e) => e.stopPropagation()} style={{ textAlign: "center" }}>
          <h2>Check Your Email</h2>
          <p style={{ color: "var(--text-muted)", fontSize: 14, marginBottom: 16 }}>
            If an account exists for that email, we've sent a login link. Click it to continue.
          </p>
          <p style={{ color: "var(--text-muted)", fontSize: 12 }}>
            The link expires in 15 minutes.
          </p>
          <button className="modal-close" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    );
  }

  // ── Password Decrypt (after magic link redeem) ──
  if (emailStep === "password-decrypt" && redeemData) {
    return (
      <div className="modal-overlay" onMouseDown={onClose}>
        <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
          <h2>Enter Password</h2>
          <p style={{ color: "var(--text-muted)", fontSize: 13, marginBottom: 16 }}>
            Enter your password to decrypt your private key.
          </p>
          <form onSubmit={handlePasswordDecrypt}>
            <div className="form-field" style={{ marginBottom: 12 }}>
              <label>Password</label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Your ncryptsec password"
                required
                autoFocus
              />
            </div>
            {error && <p style={{ color: "var(--danger)", fontSize: 13, marginBottom: 8 }}>{error}</p>}
            <button type="submit" className="btn-action" style={{ width: "100%", padding: "10px 16px" }}>
              Decrypt & Login
            </button>
          </form>
          <button className="modal-close" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    );
  }

  // ── Initial Login Options ──
  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
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

        <div style={{ borderTop: "1px solid var(--border)", margin: "12px 0", paddingTop: 12 }}>
          <button className="login-option" onClick={() => setEmailStep("login")}>
            Login with Email
          </button>
          <button className="login-option" onClick={() => setEmailStep("register")} style={{ fontSize: 13, color: "var(--text-muted)" }}>
            Register with Email
          </button>
        </div>

        <button className="modal-close" onClick={onClose}>
          Cancel
        </button>
      </div>
    </div>
  );
}

/**
 * Standalone password-decrypt modal for magic link redemption.
 * Rendered by App.tsx when a token has been redeemed.
 */
export function PasswordDecryptModal({
  redeemData,
  onDecrypt,
  onClose,
}: {
  redeemData: { npub: string; ncryptsec: string };
  onDecrypt: (pubkey: string, npub: string, nsecHex: string) => void;
  onClose: () => void;
}) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      const nsecHex = decryptNcryptsec(redeemData.ncryptsec, password);
      storeNsec(nsecHex);
      const { decodePointer } = await import("applesauce-core/helpers/pointers");
      const { data: pubkey } = decodePointer(redeemData.npub) as { type: string; data: string };
      onDecrypt(pubkey, redeemData.npub, nsecHex);
    } catch {
      setError("Wrong password. If you forgot it, your keys cannot be recovered.");
    }
  };

  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <h2>Enter Password</h2>
        <p style={{ color: "var(--text-muted)", fontSize: 13, marginBottom: 16 }}>
          Your login link was verified. Enter your password to decrypt your private key.
        </p>
        <form onSubmit={handleSubmit}>
          <div className="form-field" style={{ marginBottom: 12 }}>
            <label>Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Your ncryptsec password"
              required
              autoFocus
            />
          </div>
          {error && <p style={{ color: "var(--danger)", fontSize: 13, marginBottom: 8 }}>{error}</p>}
          <button type="submit" className="btn-action" style={{ width: "100%", padding: "10px 16px" }}>
            Decrypt & Login
          </button>
        </form>
        <button className="modal-close" onClick={onClose}>
          Cancel
        </button>
      </div>
    </div>
  );
}
