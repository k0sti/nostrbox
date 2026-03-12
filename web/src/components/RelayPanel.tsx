interface RelayInfo {
  url: string;
  connected: boolean;
}

interface RelayPanelProps {
  onClose: () => void;
}

// TODO: Pull relay state from applesauce pool / server config
const STUB_RELAYS: RelayInfo[] = [
  { url: "wss://relay.nostrbox.local", connected: false },
];

export function RelayPanel({ onClose }: RelayPanelProps) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>Relay Settings</h2>

        <ul className="relay-list">
          {STUB_RELAYS.map((r) => (
            <li key={r.url} className="relay-item">
              <span className="relay-url">{r.url}</span>
              <span
                className={`relay-status ${r.connected ? "connected" : "disconnected"}`}
                title={r.connected ? "Connected" : "Disconnected"}
              />
            </li>
          ))}
        </ul>

        <div style={{ fontSize: 13, color: "var(--text-muted)", marginTop: 12, fontStyle: "italic" }}>
          TODO: Relay management with applesauce integration
        </div>

        <button className="modal-close" onClick={onClose}>
          Close
        </button>
      </div>
    </div>
  );
}
