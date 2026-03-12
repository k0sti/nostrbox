export function RelaysView() {
  // TODO: Show relay state from applesauce pool and server config
  return (
    <div>
      <h1>Relays</h1>
      <div className="card">
        <p>Relay management will be available once applesauce integration is complete.</p>
        <p style={{ marginTop: 8, fontSize: 13, color: "var(--text-muted)" }}>
          This view will show configured relays, connection status, and sync activity.
        </p>
      </div>
    </div>
  );
}
