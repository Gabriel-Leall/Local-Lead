import { useEffect, useMemo, useState } from "react";
import { useParams } from "react-router-dom";
import { Card, GhostButton } from "../components/ui";
import { friendlyError, getSearchCells, getSearchJobs } from "../lib/api";
import type { SearchCell, SearchJob } from "../types";

function colorFor(status: string) {
  switch (status) {
    case "completed":
      return "#10b981";
    case "saturated":
      return "#8b5cf6";
    case "split":
      return "#3b82f6";
    case "running":
      return "#f59e0b";
    case "failed":
      return "#ef4444";
    default:
      return "#e5e5e5";
  }
}

export default function SearchJobPage() {
  const { id } = useParams();
  const jobId = Number(id);
  const [job, setJob] = useState<SearchJob | null>(null);
  const [cells, setCells] = useState<SearchCell[]>([]);
  const [selected, setSelected] = useState<SearchCell | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    try {
      const jobs = await getSearchJobs();
      setJob(jobs.find((j) => j.id === jobId) ?? null);
      setCells(await getSearchCells(jobId));
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  useEffect(() => {
    load();
    const t = setInterval(load, 3000);
    return () => clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [jobId]);

  const bounds = useMemo(() => {
    if (cells.length === 0) return null;
    return {
      north: Math.max(...cells.map((c) => c.north)),
      south: Math.min(...cells.map((c) => c.south)),
      east: Math.max(...cells.map((c) => c.east)),
      west: Math.min(...cells.map((c) => c.west)),
    };
  }, [cells]);

  function rect(c: SearchCell) {
    if (!bounds) return null;
    const W = 600;
    const H = 380;
    const x = ((c.west - bounds.west) / (bounds.east - bounds.west)) * W;
    const w = ((c.east - c.west) / (bounds.east - bounds.west)) * W;
    const y = ((bounds.north - c.north) / (bounds.north - bounds.south)) * H;
    const h = ((c.north - c.south) / (bounds.north - bounds.south)) * H;
    return { x, y, w: Math.max(w, 2), h: Math.max(h, 2) };
  }

  return (
    <div>
      <h1 className="text-xl font-semibold tracking-tight">
        Busca #{jobId} {job ? `· ${job.query} · ${job.city}` : ""}
      </h1>
      <p className="mt-1 text-sm text-neutral-500">
        {job ? `${job.status} · ${Math.round(job.coverage * 100)}% de cobertura (${job.completed_cells}/${job.total_cells} células)` : "Carregando…"}
      </p>
      <p className="mt-1 text-xs text-neutral-400">
        Progresso = células planejadas processadas, não % dos negócios reais.
      </p>
      {error && <p className="mt-2 text-sm text-red-600">{error}</p>}

      <div className="mt-4 grid grid-cols-[1fr_260px] gap-4">
        <Card>
          <div className="text-xs uppercase tracking-wide text-neutral-500">Células</div>
          <svg viewBox="0 0 600 380" className="mt-2 w-full rounded-lg border border-neutral-200 bg-neutral-50">
            {cells.map((c) => {
              const r = rect(c);
              if (!r) return null;
              return (
                <rect
                  key={c.id}
                  x={r.x}
                  y={r.y}
                  width={r.w}
                  height={r.h}
                  fill={colorFor(c.status)}
                  fillOpacity={c.status === "pending" ? 1 : 0.55}
                  stroke={selected?.id === c.id ? "#09090b" : "#fff"}
                  strokeWidth={selected?.id === c.id ? 2 : 1}
                  onClick={() => setSelected(c)}
                  style={{ cursor: "pointer" }}
                />
              );
            })}
          </svg>
          <div className="mt-2 flex flex-wrap gap-3 text-[11px] text-neutral-500">
            <span><i className="mr-1 inline-block h-2 w-2 rounded-sm" style={{ background: "#e5e5e5" }} />pending</span>
            <span><i className="mr-1 inline-block h-2 w-2 rounded-sm" style={{ background: "#f59e0b" }} />running</span>
            <span><i className="mr-1 inline-block h-2 w-2 rounded-sm" style={{ background: "#10b981" }} />completed</span>
            <span><i className="mr-1 inline-block h-2 w-2 rounded-sm" style={{ background: "#8b5cf6" }} />saturated</span>
            <span><i className="mr-1 inline-block h-2 w-2 rounded-sm" style={{ background: "#3b82f6" }} />split</span>
            <span><i className="mr-1 inline-block h-2 w-2 rounded-sm" style={{ background: "#ef4444" }} />failed</span>
          </div>
        </Card>
        <Card>
          <div className="text-xs uppercase tracking-wide text-neutral-500">Detalhe da célula</div>
          {!selected && <p className="mt-2 text-sm text-neutral-400">Clique numa célula.</p>}
          {selected && (
            <dl className="mt-2 space-y-1 text-sm">
              <div className="flex justify-between"><dt className="text-neutral-500">ID</dt><dd>#{selected.id}</dd></div>
              <div className="flex justify-between"><dt className="text-neutral-500">Status</dt><dd>{selected.status}</dd></div>
              <div className="flex justify-between"><dt className="text-neutral-500">Depth</dt><dd>{selected.depth}</dd></div>
              <div className="flex justify-between"><dt className="text-neutral-500">Results</dt><dd>{selected.raw_result_count}</dd></div>
              <div className="flex justify-between"><dt className="text-neutral-500">Unique</dt><dd>{selected.new_unique_count}</dd></div>
              <div className="flex justify-between"><dt className="text-neutral-500">Provider</dt><dd>{selected.provider}</dd></div>
              <div className="mt-2"><GhostButton onClick={() => setSelected(null)}>Clear</GhostButton></div>
            </dl>
          )}
        </Card>
      </div>
    </div>
  );
}
