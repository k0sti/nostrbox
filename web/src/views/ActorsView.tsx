import { useEffect, useState } from "react";
import { ops, type Actor } from "../lib/api";

function shortenPubkey(pk: string): string {
  if (pk.length <= 16) return pk;
  return `${pk.slice(0, 8)}...${pk.slice(-4)}`;
}

export function ActorsView() {
  const [actors, setActors] = useState<Actor[]>([]);

  useEffect(() => {
    ops.actorList().then((res) => {
      if (res.ok && res.data) setActors(res.data);
    });
  }, []);

  return (
    <div>
      <h1>Actors</h1>
      {actors.length === 0 ? (
        <div className="card">No actors registered yet.</div>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>Pubkey</th>
              <th>Name</th>
              <th>Kind</th>
              <th>Role</th>
              <th>Groups</th>
            </tr>
          </thead>
          <tbody>
            {actors.map((a) => (
              <tr key={a.pubkey}>
                <td className="pubkey-short">{shortenPubkey(a.pubkey)}</td>
                <td>{a.display_name || "—"}</td>
                <td>
                  <span className="badge badge-member">{a.kind}</span>
                </td>
                <td>
                  <span className={`badge badge-${a.global_role}`}>
                    {a.global_role}
                  </span>
                </td>
                <td>{a.groups.length}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
