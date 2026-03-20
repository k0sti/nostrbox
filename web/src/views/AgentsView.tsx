import { useEffect, useState } from "react";
import { ops, type Actor } from "../lib/api";
import { PubkeyCell } from "../components/PubkeyCell";

export function AgentsView() {
  const [agents, setAgents] = useState<Actor[]>([]);

  useEffect(() => {
    ops.actorList().then((res) => {
      if (res.ok && res.data) {
        setAgents(res.data.filter((a) => a.kind === "agent"));
      }
    });
  }, []);

  return (
    <div>
      <h1>Agents</h1>
      {agents.length === 0 ? (
        <div className="card">No agents registered yet.</div>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>Identity</th>
              <th>Role</th>
              <th>Status</th>
              <th>Groups</th>
            </tr>
          </thead>
          <tbody>
            {agents.map((a) => (
              <tr key={a.pubkey}>
                <td>
                  <PubkeyCell
                    pubkey={a.pubkey}
                    npub={a.npub}
                    displayName={a.display_name}
                  />
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
  );
}
