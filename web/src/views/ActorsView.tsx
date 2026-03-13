import { useEffect, useState } from "react";
import { ops, type Actor, type ActorDetail } from "../lib/api";
import { PubkeyCell } from "../components/PubkeyCell";

export function ActorsView() {
  const [actors, setActors] = useState<Actor[]>([]);
  const [selected, setSelected] = useState<ActorDetail | null>(null);

  useEffect(() => {
    ops.actorList().then((res) => {
      if (res.ok && res.data) setActors(res.data);
    });
  }, []);

  const handleSelect = async (pubkey: string) => {
    const res = await ops.actorDetail(pubkey);
    if (res.ok && res.data) setSelected(res.data);
  };

  return (
    <div>
      <h1>Actors</h1>
      <div style={{ display: "flex", gap: 24 }}>
        <div style={{ flex: 1 }}>
          {actors.length === 0 ? (
            <div className="card">No actors registered yet.</div>
          ) : (
            <table className="data-table">
              <thead>
                <tr>
                  <th>Identity</th>
                  <th>Kind</th>
                  <th>Role</th>
                  <th>Status</th>
                  <th>Groups</th>
                </tr>
              </thead>
              <tbody>
                {actors.map((a) => (
                  <tr
                    key={a.pubkey}
                    onClick={() => handleSelect(a.pubkey)}
                    className="clickable-row"
                  >
                    <td>
                      <PubkeyCell
                        pubkey={a.pubkey}
                        npub={a.npub}
                        displayName={a.display_name}
                      />
                    </td>
                    <td>
                      <span className="badge badge-member">{a.kind}</span>
                    </td>
                    <td>
                      <span className={`badge badge-${a.global_role}`}>
                        {a.global_role}
                      </span>
                    </td>
                    <td>
                      <span className={`badge badge-${a.status}`}>
                        {a.status}
                      </span>
                    </td>
                    <td>{a.groups.length}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {selected && (
          <div className="detail-panel">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
              <h2 style={{ fontSize: 18, color: "var(--text-heading)" }}>Actor Detail</h2>
              <button className="modal-close" style={{ width: "auto", padding: "4px 12px" }} onClick={() => setSelected(null)}>
                Close
              </button>
            </div>

            <div style={{ marginBottom: 16 }}>
              <PubkeyCell
                pubkey={selected.pubkey}
                npub={selected.npub}
                displayName={selected.display_name}
              />
            </div>

            <div className="detail-field">
              <span className="detail-label">Role</span>
              <span className={`badge badge-${selected.global_role}`}>
                {selected.global_role}
              </span>
            </div>
            <div className="detail-field">
              <span className="detail-label">Status</span>
              <span className={`badge badge-${selected.status}`}>
                {selected.status}
              </span>
            </div>
            {selected.registration_status && (
              <div className="detail-field">
                <span className="detail-label">Registration</span>
                <span className={`badge badge-${selected.registration_status}`}>
                  {selected.registration_status}
                </span>
              </div>
            )}

            {selected.group_details.length > 0 && (
              <>
                <h3 style={{ fontSize: 14, color: "var(--text-muted)", marginTop: 16, marginBottom: 8, textTransform: "uppercase" }}>
                  Groups
                </h3>
                {selected.group_details.map((g) => (
                  <div key={g.group_id} className="detail-field">
                    <span>{g.group_name}</span>
                    <span className={`badge badge-${g.role}`}>{g.role}</span>
                  </div>
                ))}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
