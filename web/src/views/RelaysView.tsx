import { useState, useEffect } from "react";

const API_BASE = import.meta.env.VITE_API_URL ?? "";

interface RelayInfoDoc {
  name?: string;
  description?: string;
  supported_nips?: number[];
  software?: string;
  version?: string;
  relay_url?: string;
}

export function RelaysView() {
  const [info, setInfo] = useState<RelayInfoDoc | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch(`${API_BASE}/api/relay-info`, {
      headers: { Accept: "application/nostr+json" },
    })
      .then((r) => r.json())
      .then((data) => {
        setInfo(data);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div>
        <h1>Relays</h1>
        <p style={{ color: "var(--text-muted)" }}>Loading...</p>
      </div>
    );
  }

  return (
    <div>
      <h1>Relays</h1>
      {info ? (
        <div className="card">
          <table>
            <tbody>
              <tr>
                <td style={{ fontWeight: 600, paddingRight: 16 }}>Name</td>
                <td>{info.name ?? "—"}</td>
              </tr>
              <tr>
                <td style={{ fontWeight: 600, paddingRight: 16 }}>URL</td>
                <td style={{ wordBreak: "break-all" }}>
                  {info.relay_url ?? "—"}
                </td>
              </tr>
              <tr>
                <td style={{ fontWeight: 600, paddingRight: 16 }}>Software</td>
                <td>
                  {info.software ?? "—"} {info.version ?? ""}
                </td>
              </tr>
              <tr>
                <td style={{ fontWeight: 600, paddingRight: 16 }}>
                  Description
                </td>
                <td>{info.description ?? "—"}</td>
              </tr>
              <tr>
                <td style={{ fontWeight: 600, paddingRight: 16 }}>
                  Supported NIPs
                </td>
                <td>{info.supported_nips?.join(", ") ?? "—"}</td>
              </tr>
            </tbody>
          </table>
        </div>
      ) : (
        <div className="card">
          <p style={{ color: "var(--text-muted)" }}>
            Could not fetch relay information.
          </p>
        </div>
      )}
    </div>
  );
}
