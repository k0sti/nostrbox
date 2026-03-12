import { useEffect, useState } from "react";
import { ops, type Group } from "../lib/api";

export function GroupsView() {
  const [groups, setGroups] = useState<Group[]>([]);

  useEffect(() => {
    ops.groupList().then((res) => {
      if (res.ok && res.data) setGroups(res.data);
    });
  }, []);

  return (
    <div>
      <h1>Groups</h1>
      {groups.length === 0 ? (
        <div className="card">No groups yet.</div>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>ID</th>
              <th>Visibility</th>
              <th>Members</th>
            </tr>
          </thead>
          <tbody>
            {groups.map((g) => (
              <tr key={g.group_id}>
                <td style={{ fontWeight: 500 }}>{g.name}</td>
                <td className="pubkey-short">{g.group_id}</td>
                <td>
                  <span className={`badge badge-${g.visibility}`}>
                    {g.visibility}
                  </span>
                </td>
                <td>{g.members.length}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
