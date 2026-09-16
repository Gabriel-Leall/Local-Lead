import { useEffect, useState } from "react";
import { Stat } from "../components/ui";
import { getDashboardStats } from "../lib/api";
import type { DashboardStats } from "../types";

export default function DashboardPage() {
  const [stats, setStats] = useState<DashboardStats | null>(null);

  useEffect(() => {
    getDashboardStats().then(setStats).catch(() => setStats(null));
  }, []);

  return (
    <div>
      <h1 className="text-xl font-semibold tracking-tight">Dashboard</h1>
      <p className="mt-1 text-sm text-neutral-500">Counts come from the local database — no mocks.</p>
      <div className="mt-6 grid grid-cols-3 gap-4">
        <Stat label="Total leads" value={stats?.total_leads ?? "—"} />
        <Stat label="New leads" value={stats?.new_leads ?? "—"} />
        <Stat label="Searches completed" value={stats?.searches_completed ?? "—"} />
        <Stat label="Qualified" value={stats?.qualified ?? "—"} />
        <Stat label="Contacted" value={stats?.contacted ?? "—"} />
        <Stat label="Replied+" value={stats?.replied ?? "—"} />
        <Stat label="Won" value={stats?.won ?? "—"} />
        <Stat label="Without website" value={stats?.without_website ?? "—"} />
        <Stat label="With email" value={stats?.with_email ?? "—"} />
      </div>
    </div>
  );
}
