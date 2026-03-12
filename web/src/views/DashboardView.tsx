import { useEffect, useState } from "react";
import { ops, type DashboardSummary } from "../lib/api";

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
    </div>
  );
}
