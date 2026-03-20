import { useState, useEffect } from "react";
import { loadSettings, saveSettings, resetSettings, getDefaults, type AppSettings } from "../lib/settings";
import { isTransportConnected, connectTransport, disconnectTransport, getLastConnectError } from "../lib/contextvm";
import { pubkeyToNpub, compressNpub } from "../lib/nostr";

const API_BASE = import.meta.env.VITE_API_URL ?? "";

interface RelayInfo {
  relay_url?: string;
  pubkey?: string;
  status?: string;
  name?: string;
  description?: string;
  supported_nips?: number[];
  software?: string;
  version?: string;
}

interface RelayPanelProps {
  onClose: () => void;
}

export function RelayPanel({ onClose }: RelayPanelProps) {
  const [relayInfo, setRelayInfo] = useState<RelayInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [settings, setSettings] = useState<AppSettings>(loadSettings);
  const [saved, setSaved] = useState(false);
  const [cvmStatus, setCvmStatus] = useState<"disconnected" | "connecting" | "connected">(
    isTransportConnected() ? "connected" : "disconnected"
  );
  const [connectError, setConnectError] = useState<string | null>(getLastConnectError());

  useEffect(() => {
    fetch(`${API_BASE}/api/relay-info`, {
      headers: { Accept: "application/nostr+json" },
    })
      .then((r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return r.json();
      })
      .then((data) => {
        setRelayInfo(data);
        setLoading(false);
      })
      .catch((e) => {
        setError(e.message || "Network error");
        setLoading(false);
      });
  }, []);

  const handleSave = async () => {
    saveSettings(settings);

    // If switching to CVM, attempt to connect transport
    if (settings.transport === "cvm" && !isTransportConnected()) {
      setCvmStatus("connecting");
      setConnectError(null);
      // Disconnect first so it reconnects with new settings
      await disconnectTransport();
      const ok = await connectTransport();
      setCvmStatus(ok ? "connected" : "disconnected");
      if (!ok) setConnectError(getLastConnectError());
    }

    // If switching to HTTP, we can leave transport connected (no harm)
    // but update status display
    if (settings.transport === "http") {
      setCvmStatus(isTransportConnected() ? "connected" : "disconnected");
    }

    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  };

  const handleReset = () => {
    const defaults = getDefaults();
    setSettings(defaults);
    resetSettings();
    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  };

  const activeTransport = (() => {
    const current = loadSettings();
    if (current.transport === "cvm" && isTransportConnected()) return "cvm";
    return "http";
  })();

  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <h2>Settings</h2>

        {/* Active transport indicator */}
        <div className="settings-status">
          <span className={`relay-status ${activeTransport === "cvm" ? "connected" : "connected"}`} />
          <div>
            <div style={{ fontSize: 13, color: "var(--text-muted)" }}>
              Using <strong>{activeTransport === "cvm" ? "ContextVM (Nostr)" : "HTTP"}</strong>
              {settings.transport === "cvm" && !isTransportConnected() && cvmStatus !== "connecting" && (
                <span style={{ color: "var(--danger)" }}> — transport not connected</span>
              )}
              {cvmStatus === "connecting" && (
                <span> — connecting...</span>
              )}
            </div>
            {connectError && settings.transport === "cvm" && (
              <div style={{ fontSize: 12, color: "var(--danger)", marginTop: 4, fontFamily: "monospace" }}>
                {connectError}
              </div>
            )}
          </div>
        </div>

        {/* Transport Mode */}
        <div className="settings-section">
          <h3 className="settings-heading">Transport</h3>
          <div className="settings-field">
            <label className="settings-label">Connection mode</label>
            <div className="toggle-group">
              <button
                className={`toggle-btn ${settings.transport === "http" ? "active" : ""}`}
                onClick={() => setSettings({ ...settings, transport: "http" })}
              >
                HTTP
              </button>
              <button
                className={`toggle-btn ${settings.transport === "cvm" ? "active" : ""}`}
                onClick={() => setSettings({ ...settings, transport: "cvm" })}
              >
                ContextVM
              </button>
            </div>
            <span className="settings-hint">
              {settings.transport === "http"
                ? "REST API calls over HTTP (default)"
                : "JSON-RPC over Nostr events (requires NIP-07 signer)"}
            </span>
          </div>
        </div>

        {/* Relay Address */}
        <div className="settings-section">
          <h3 className="settings-heading">Relay</h3>
          <div className="settings-field">
            <label className="settings-label">Relay address</label>
            <input
              type="text"
              value={settings.relayUrl}
              onChange={(e) => setSettings({ ...settings, relayUrl: e.target.value })}
              placeholder={relayInfo?.relay_url || "wss://..."}
            />
            <span className="settings-hint">
              Leave empty to use server default{relayInfo?.relay_url ? ` (${relayInfo.relay_url})` : ""}
            </span>
          </div>
        </div>

        {/* Server Info */}
        <div className="settings-section">
          <h3 className="settings-heading">Server Info</h3>
          {loading ? (
            <p style={{ color: "var(--text-muted)", fontSize: 13 }}>Loading...</p>
          ) : error ? (
            <p style={{ color: "var(--text-muted)", fontSize: 13 }}>
              Could not fetch relay info: {error}
            </p>
          ) : relayInfo ? (
            <div style={{ fontSize: 13, color: "var(--text-muted)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                <span className="relay-url">{relayInfo.relay_url ?? "—"}</span>
                <span
                  className={`relay-status ${relayInfo.status === "running" || relayInfo.relay_url ? "connected" : "disconnected"}`}
                />
              </div>
              {relayInfo.name && (
                <div>{relayInfo.name}{relayInfo.version ? ` v${relayInfo.version}` : ""}</div>
              )}
              {relayInfo.description && <div>{relayInfo.description}</div>}
              {relayInfo.pubkey && (
                <div style={{ marginTop: 4 }}>
                  <span style={{ color: "var(--text-muted)" }}>npub: </span>
                  <span className="relay-url">{compressNpub(pubkeyToNpub(relayInfo.pubkey))}</span>
                </div>
              )}
              {relayInfo.supported_nips && relayInfo.supported_nips.length > 0 && (
                <div style={{ marginTop: 4 }}>NIPs: {relayInfo.supported_nips.join(", ")}</div>
              )}
            </div>
          ) : (
            <p style={{ color: "var(--text-muted)", fontSize: 13 }}>No relay information available.</p>
          )}
        </div>

        {/* Action buttons */}
        <div className="settings-actions">
          <button className="btn-action" onClick={handleSave}>
            {saved ? "Saved!" : "Save"}
          </button>
          <button className="btn-action btn-secondary" onClick={handleReset}>
            Reset to Defaults
          </button>
          <button className="modal-close" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
