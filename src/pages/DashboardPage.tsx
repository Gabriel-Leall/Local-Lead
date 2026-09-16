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
      <h1 className="text-xl font-semibold tracking-tight">Painel</h1>
      <p className="mt-1 text-sm text-neutral-500">Números reais do banco local — sem simulação.</p>
      <div className="mt-6 grid grid-cols-3 gap-4">
        <Stat label="Total de leads" value={stats?.total_leads ?? "—"} />
        <Stat label="Novos" value={stats?.new_leads ?? "—"} />
        <Stat label="Buscas concluídas" value={stats?.searches_completed ?? "—"} />
        <Stat label="Qualificados" value={stats?.qualified ?? "—"} />
        <Stat label="Contatados" value={stats?.contacted ?? "—"} />
        <Stat label="Responderam+" value={stats?.replied ?? "—"} />
        <Stat label="Ganhos" value={stats?.won ?? "—"} />
        <Stat label="Sem site" value={stats?.without_website ?? "—"} />
        <Stat label="Com e-mail" value={stats?.with_email ?? "—"} />
      </div>
    </div>
  );
}
