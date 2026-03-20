import { useEffect, useState } from "react";
import { ops, type EmailIdentity } from "../lib/api";
import { PubkeyCell } from "../components/PubkeyCell";

function formatDate(ts: number | null): string {
  if (!ts) return "Never";
  return new Date(ts * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function EmailAccountsView() {
  const [identities, setIdentities] = useState<EmailIdentity[]>([]);
  const [deleting, setDeleting] = useState<number | null>(null);

  const load = () => {
    ops.emailList().then((res) => {
      if (res.ok && res.data) setIdentities(res.data);
    });
  };

  useEffect(() => { load(); }, []);

  const handleDelete = async (id: number, email: string) => {
    if (!confirm(`Delete email identity for ${email}? This also removes all login tokens.`)) return;
    setDeleting(id);
    const res = await ops.emailDelete(id);
    if (res.ok) {
      setIdentities((prev) => prev.filter((e) => e.id !== id));
    }
    setDeleting(null);
  };

  return (
    <div>
      <h1>Email Accounts</h1>
      {identities.length === 0 ? (
        <div className="card">No email accounts registered yet.</div>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>Identity</th>
              <th>Email</th>
              <th>Key</th>
              <th>Role</th>
              <th>Created</th>
              <th>Last Login</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {identities.map((ei) => (
              <tr key={ei.id}>
                <td>
                  <PubkeyCell
                    pubkey={ei.pubkey}
                    npub={ei.npub ?? undefined}
                    displayName={ei.display_name}
                  />
                </td>
                <td style={{ fontSize: 13 }}>{ei.email}</td>
                <td>
                  <span className={`badge ${ei.has_key ? "badge-active" : "badge-disabled"}`}>
                    {ei.has_key ? "active" : "cleared"}
                  </span>
                </td>
                <td>
                  {ei.global_role ? (
                    <span className={`badge badge-${ei.global_role}`}>{ei.global_role}</span>
                  ) : (
                    <span style={{ color: "var(--text-muted)" }}>—</span>
                  )}
                </td>
                <td style={{ fontSize: 12, color: "var(--text-muted)" }}>{formatDate(ei.created_at)}</td>
                <td style={{ fontSize: 12, color: "var(--text-muted)" }}>{formatDate(ei.last_login_at)}</td>
                <td>
                  <button
                    className="btn-action btn-danger"
                    style={{ fontSize: 11, padding: "3px 8px" }}
                    onClick={() => handleDelete(ei.id, ei.email)}
                    disabled={deleting === ei.id}
                  >
                    {deleting === ei.id ? "..." : "Delete"}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
