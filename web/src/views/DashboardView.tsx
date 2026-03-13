import { useEffect, useState } from "react";
import { ops, type DashboardSummary } from "../lib/api";

const ROLE_LABELS: Record<string, string> = {
  owner: "Owners",
  admin: "Admins",
  member: "Members",
  guest: "Guests",
};

export function DashboardView() {
  const [data, setData] = useState<DashboardSummary | null>(null);

  useEffect(() => {
    ops.dashboardGet().then((res) => {
      if (res.ok && res.data) setData(res.data);
    });
  }, []);

  return (
    <div>
      <h1>Dashboard</h1>
      <div className="stat-grid">
        <div className="stat-card">
          <div className="stat-value">{data?.pending_registrations ?? "—"}</div>
          <div className="stat-label">Pending Registrations</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">{data?.total_actors ?? "—"}</div>
          <div className="stat-label">Total Actors</div>
        </div>
        <div className="stat-card">
          <div className="stat-value">{data?.total_groups ?? "—"}</div>
          <div className="stat-label">Total Groups</div>
        </div>
      </div>

      {data?.actors_by_role && Object.keys(data.actors_by_role).length > 0 && (
        <>
          <h2 style={{ fontSize: 18, marginBottom: 12, color: "var(--text-heading)" }}>
            Actors by Role
          </h2>
          <div className="stat-grid">
            {Object.entries(data.actors_by_role).map(([role, count]) => (
              <div key={role} className="stat-card">
                <div className="stat-value">{count}</div>
                <div className="stat-label">
                  <span className={`badge badge-${role}`}>{ROLE_LABELS[role] ?? role}</span>
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
