import { useEffect, useState } from "react";
import { ops, type Registration } from "../lib/api";
import { PubkeyCell } from "../components/PubkeyCell";

export function RegistrationsView() {
  const [regs, setRegs] = useState<Registration[]>([]);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const load = () => {
    ops.registrationList().then((res) => {
      if (res.ok && res.data) setRegs(res.data);
    });
  };

  useEffect(() => { load(); }, []);

  const handleApprove = async (pubkey: string) => {
    await ops.registrationApprove(pubkey);
    load();
  };

  const handleDeny = async (pubkey: string) => {
    await ops.registrationDeny(pubkey);
    load();
  };

  return (
    <div>
      <h1>Registration Requests</h1>
      {regs.length === 0 ? (
        <div className="card">No registration requests.</div>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>Identity</th>
              <th>Message</th>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {regs.map((r) => (
              <tr key={r.pubkey}>
                <td><PubkeyCell pubkey={r.pubkey} /></td>
                <td>{r.message || "—"}</td>
                <td>
                  <span className={`badge badge-${r.status}`}>
                    {r.status}
                  </span>
                </td>
                <td>
                  <div style={{ display: "flex", gap: 6 }}>
                    {r.status === "pending" && (
                      <>
                        <button
                          className="btn-action"
                          onClick={() => handleApprove(r.pubkey)}
                        >
                          Approve
                        </button>
                        <button
                          className="btn-action btn-danger"
                          onClick={() => handleDeny(r.pubkey)}
                        >
                          Deny
                        </button>
                      </>
                    )}
                    <button
                      className="btn-action btn-danger"
                      style={{ fontSize: 11, padding: "3px 8px" }}
                      disabled={deletingId === r.pubkey}
                      onClick={async () => {
                        if (!confirm(`Delete registration for ${r.pubkey.slice(0, 12)}...?`)) return;
                        setDeletingId(r.pubkey);
                        const res = await ops.registrationDelete(r.pubkey);
                        if (res.ok) load();
                        setDeletingId(null);
                      }}
                    >
                      {deletingId === r.pubkey ? "..." : <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6"/><path d="M14 11v6"/><path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"/></svg>}
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
