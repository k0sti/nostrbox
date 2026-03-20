import { useEffect, useState } from "react";
import { ops, type Group } from "../lib/api";
import { PubkeyCell } from "../components/PubkeyCell";

export function GroupsView() {
  const [groups, setGroups] = useState<Group[]>([]);
  const [selected, setSelected] = useState<Group | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [form, setForm] = useState({ group_id: "", name: "", description: "", visibility: "group" });

  const load = () => {
    ops.groupList().then((res) => {
      if (res.ok && res.data) setGroups(res.data);
    });
  };

  useEffect(() => { load(); }, []);

  const handleSelect = async (groupId: string) => {
    const res = await ops.groupGet(groupId);
    if (res.ok && res.data) setSelected(res.data);
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    await ops.groupPut(form);
    setShowCreate(false);
    setForm({ group_id: "", name: "", description: "", visibility: "group" });
    load();
  };

  const handleRemoveMember = async (groupId: string, pubkey: string) => {
    await ops.groupRemoveMember(groupId, pubkey);
    handleSelect(groupId);
    load();
  };

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <h1 style={{ marginBottom: 0 }}>Groups</h1>
        <button className="btn-action" onClick={() => setShowCreate(!showCreate)}>
          {showCreate ? "Cancel" : "Create Group"}
        </button>
      </div>

      {showCreate && (
        <form className="card" onSubmit={handleCreate} style={{ marginBottom: 16 }}>
          <div className="form-grid">
            <div className="form-field">
              <label>Group ID</label>
              <input
                value={form.group_id}
                onChange={(e) => setForm({ ...form, group_id: e.target.value })}
                placeholder="my-group"
                required
              />
            </div>
            <div className="form-field">
              <label>Name</label>
              <input
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                placeholder="My Group"
                required
              />
            </div>
            <div className="form-field">
              <label>Description</label>
              <input
                value={form.description}
                onChange={(e) => setForm({ ...form, description: e.target.value })}
                placeholder="A description..."
              />
            </div>
            <div className="form-field">
              <label>Visibility</label>
              <select
                value={form.visibility}
                onChange={(e) => setForm({ ...form, visibility: e.target.value })}
              >
                <option value="public">Public</option>
                <option value="group">Group</option>
                <option value="internal">Internal</option>
              </select>
            </div>
          </div>
          <button type="submit" className="btn-action" style={{ marginTop: 12 }}>
            Create
          </button>
        </form>
      )}

      <div style={{ display: "flex", gap: 24 }}>
        <div style={{ flex: 1 }}>
          {groups.length === 0 ? (
            <div className="card">No groups yet.</div>
          ) : (
            <table className="data-table">
              <thead>
                <tr>
                  <th>Name</th>
                  <th>ID</th>
                  <th>Visibility</th>
                  <th>Policy</th>
                  <th>Status</th>
                  <th>Members</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {groups.map((g) => (
                  <tr
                    key={g.group_id}
                    onClick={() => handleSelect(g.group_id)}
                    className="clickable-row"
                  >
                    <td style={{ fontWeight: 500 }}>{g.name}</td>
                    <td className="pubkey-short">{g.group_id.length > 16 ? `${g.group_id.slice(0, 8)}...${g.group_id.slice(-4)}` : g.group_id}</td>
                    <td>
                      <span className={`badge badge-${g.visibility}`}>
                        {g.visibility}
                      </span>
                    </td>
                    <td>
                      <span className="badge badge-member">{g.join_policy}</span>
                    </td>
                    <td>
                      <span className={`badge badge-${g.status}`}>{g.status}</span>
                    </td>
                    <td>{g.members.length}</td>
                    <td>
                      <button
                        className="btn-action btn-danger"
                        style={{ fontSize: 11, padding: "3px 8px" }}
                        disabled={deletingId === g.group_id}
                        onClick={async (e) => {
                          e.stopPropagation();
                          if (!confirm(`Delete group ${g.name}?`)) return;
                          setDeletingId(g.group_id);
                          const res = await ops.groupDelete(g.group_id);
                          if (res.ok) load();
                          setDeletingId(null);
                        }}
                      >
                        {deletingId === g.group_id ? "..." : "Delete"}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {selected && (
          <div className="detail-panel">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
              <h2 style={{ fontSize: 18, color: "var(--text-heading)" }}>Group Detail</h2>
              <button className="modal-close" style={{ width: "auto", padding: "4px 12px" }} onClick={() => setSelected(null)}>
                Close
              </button>
            </div>

            <div className="detail-field">
              <span className="detail-label">Name</span>
              <span>{selected.name}</span>
            </div>
            <div className="detail-field">
              <span className="detail-label">ID</span>
              <span className="pubkey-short">{selected.group_id}</span>
            </div>
            {selected.slug && (
              <div className="detail-field">
                <span className="detail-label">Slug</span>
                <span>{selected.slug}</span>
              </div>
            )}
            <div className="detail-field">
              <span className="detail-label">Description</span>
              <span>{selected.description || "—"}</span>
            </div>
            <div className="detail-field">
              <span className="detail-label">Visibility</span>
              <span className={`badge badge-${selected.visibility}`}>{selected.visibility}</span>
            </div>
            <div className="detail-field">
              <span className="detail-label">Join Policy</span>
              <span className="badge badge-member">{selected.join_policy}</span>
            </div>
            <div className="detail-field">
              <span className="detail-label">Status</span>
              <span className={`badge badge-${selected.status}`}>{selected.status}</span>
            </div>

            <h3 style={{ fontSize: 14, color: "var(--text-muted)", marginTop: 16, marginBottom: 8, textTransform: "uppercase" }}>
              Members ({selected.members.length})
            </h3>
            {selected.members.length === 0 ? (
              <div style={{ fontSize: 13, color: "var(--text-muted)", fontStyle: "italic" }}>
                No members yet.
              </div>
            ) : (
              selected.members.map((m) => (
                <div key={m.pubkey} style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "6px 0", borderBottom: "1px solid var(--border)" }}>
                  <PubkeyCell pubkey={m.pubkey} />
                  <div style={{ display: "flex", gap: 8, alignItems: "center", flexShrink: 0 }}>
                    <span className={`badge badge-${m.role}`}>{m.role}</span>
                    <button
                      className="btn-action btn-danger"
                      style={{ padding: "2px 8px", fontSize: 11 }}
                      onClick={(e) => { e.stopPropagation(); handleRemoveMember(selected.group_id, m.pubkey); }}
                    >
                      Remove
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>
        )}
      </div>
    </div>
  );
}
