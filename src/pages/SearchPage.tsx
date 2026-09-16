import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Button, Card, GhostButton, Input, Label } from "../components/ui";
import {
  cancelJob,
  checkScraperBinary,
  friendlyError,
  getProviderCounts,
  getSearchJobs,
  importScraperJson,
  pauseJob,
  resumeJob,
  searchLeads,
  startAdaptiveSearch,
} from "../lib/api";
import { secretStore } from "../lib/secretStore";
import type { SearchJob } from "../types";

export default function SearchPage() {
  const [query, setQuery] = useState("Dentist");
  const [city, setCity] = useState("Miami, Florida");
  const [radiusKm, setRadiusKm] = useState(30);
  const [strategy, setStrategy] = useState<"single" | "adaptive">("adaptive");
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [jobs, setJobs] = useState<SearchJob[]>([]);
  const [scraperMsg, setScraperMsg] = useState<string | null>(null);
  const [binaryMsg, setBinaryMsg] = useState<string | null>(null);
  const [counts, setCounts] = useState<Record<number, string>>({});

  async function loadJobs() {
    try {
      setJobs(await getSearchJobs());
    } catch {
      /* ignore before first db init */
    }
  }

  useEffect(() => {
    loadJobs();
  }, []);

  async function onSearch() {
    setLoading(true);
    setMessage(null);
    setError(null);
    try {
      const apiKey = await secretStore.getApiKey();
      if (!apiKey) {
        setError("Save your Google Places API key in Settings first.");
        return;
      }
      if (strategy === "single") {
        const r = await searchLeads({ query, city, radiusKm, apiKey });
        setMessage(`Initial search results: ${r.result_count} found, ${r.new_count} new.`);
      } else {
        const r = await startAdaptiveSearch({ query, city, radiusKm, apiKey });
        setMessage(`Adaptive coverage finished. Job #${r.job_id} — see progress below.`);
      }
      await loadJobs();
    } catch (e) {
      setError(friendlyError(e));
    } finally {
      setLoading(false);
    }
  }

  async function doPause(id: number) {
    await pauseJob(id);
    loadJobs();
  }

  async function doResume(id: number) {
    const apiKey = await secretStore.getApiKey();
    await resumeJob(id, apiKey);
    loadJobs();
  }

  async function doCancel(id: number) {
    await cancelJob(id);
    loadJobs();
  }

  return (
    <div>
      <h1 className="text-xl font-semibold tracking-tight">New search</h1>
      <p className="mt-1 text-sm text-neutral-500">
        Single = one Text Search. Adaptive = quadtree coverage with systematic cells.
      </p>
      <Card>
        <div className="mt-2 grid gap-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <Label>Niche / Query</Label>
              <Input value={query} onChange={(e) => setQuery(e.target.value)} />
            </div>
            <div>
              <Label>City</Label>
              <Input value={city} onChange={(e) => setCity(e.target.value)} />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <Label>Radius (km)</Label>
              <Input type="number" min={1} max={100} value={radiusKm} onChange={(e) => setRadiusKm(Number(e.target.value))} />
            </div>
            <div>
              <Label>Strategy</Label>
              <div className="flex gap-2">
                {(["single", "adaptive"] as const).map((s) => (
                  <GhostButton
                    key={s}
                    onClick={() => setStrategy(s)}
                    className={strategy === s ? "border-neutral-900 bg-neutral-900 text-white hover:bg-neutral-900" : ""}
                  >
                    {s === "single" ? "Single" : "Adaptive Coverage"}
                  </GhostButton>
                ))}
              </div>
            </div>
          </div>
          <div>
            <Button onClick={onSearch} disabled={loading}>
              {loading ? "Searching…" : strategy === "adaptive" ? "Start adaptive search" : "Start search"}
            </Button>
          </div>
          {message && <p className="text-sm text-neutral-700">{message}</p>}
          {error && <p className="text-sm text-red-600">{error}</p>}
        </div>
      </Card>

      <h2 className="mt-8 text-sm font-semibold uppercase tracking-wide text-neutral-500">Maps scraper provider</h2>
      <Card>
        <p className="text-xs text-neutral-500">
          Alternative to Places API. Export JSON from gosom/google-maps-scraper, then import here. Cross-provider dedup by website → phone → name+coords.
        </p>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <GhostButton onClick={async () => {
            const b = await checkScraperBinary();
            setBinaryMsg(`${b.available ? "✓" : "—"} ${b.message}`);
          }}>Check binary</GhostButton>
          <label className="cursor-pointer rounded-lg border border-neutral-200 bg-white px-3 py-1.5 text-sm text-neutral-700 hover:bg-neutral-100">
            Import JSON
            <input type="file" accept=".json,application/json" className="hidden" onChange={async (e) => {
              const f = e.target.files?.[0];
              if (!f) return;
              const text = await f.text();
              try {
                const r = await importScraperJson(query, city, text);
                setScraperMsg(`Imported ${r.imported} new, merged ${r.merged}, skipped ${r.skipped}. Job #${r.job_id}.`);
                loadJobs();
              } catch (err) {
                setScraperMsg(friendlyError(err));
              }
            }} />
          </label>
          {binaryMsg && <span className="text-xs text-neutral-500">{binaryMsg}</span>}
        </div>
        {scraperMsg && <p className="mt-2 text-sm text-neutral-700">{scraperMsg}</p>}
      </Card>

      <h2 className="mt-8 text-sm font-semibold uppercase tracking-wide text-neutral-500">Recent jobs</h2>
      <div className="mt-3 grid gap-3">
        {jobs.map((j) => (
          <Card key={j.id}>
            <div className="flex items-center justify-between gap-4">
              <div>
                <Link to={`/jobs/${j.id}`} className="text-sm font-medium hover:underline">
                  #{j.id} {j.query} · {j.city}
                </Link>
                <div className="mt-1 text-xs text-neutral-500">
                  {j.strategy} · {j.status} · {j.result_count} results / {j.new_count} new ·{" "}
                  {j.total_cells > 0 ? `${j.completed_cells}/${j.total_cells} cells · ${Math.round(j.coverage * 100)}%` : "single query"}
                </div>
                {j.total_cells > 0 && (
                  <div className="mt-2 h-1.5 w-64 overflow-hidden rounded-full bg-neutral-200">
                    <div className="h-full bg-neutral-900" style={{ width: `${Math.round(j.coverage * 100)}%` }} />
                  </div>
                )}
                <button
                  className="mt-1 text-xs text-neutral-400 hover:underline"
                  onClick={async () => {
                    try {
                      const c = await getProviderCounts(j.id);
                      setCounts((p) => ({ ...p, [j.id]: c.map((x) => `${x.provider}: ${x.leads}`).join(" · ") || "no providers" }));
                    } catch (err) {
                      setCounts((p) => ({ ...p, [j.id]: friendlyError(err) }));
                    }
                  }}
                >
                  {counts[j.id] ?? "Show providers"}
                </button>
              </div>
              <div className="flex gap-1">
                {(j.status === "running" || j.status === "pending") && (
                  <GhostButton onClick={() => doPause(j.id)}>Pause</GhostButton>
                )}
                {(j.status === "paused" || j.status === "interrupted" || j.status === "failed") && (
                  <GhostButton onClick={() => doResume(j.id)}>Resume</GhostButton>
                )}
                {j.status !== "cancelled" && j.status !== "completed" && (
                  <GhostButton onClick={() => doCancel(j.id)}>Cancel</GhostButton>
                )}
              </div>
            </div>
          </Card>
        ))}
        {jobs.length === 0 && <p className="text-sm text-neutral-400">No searches yet.</p>}
      </div>
    </div>
  );
}
