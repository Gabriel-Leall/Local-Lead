import { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import LeadMap from "../components/LeadMap";
import { Button, Card, GhostButton, Input, Label } from "../components/ui";
import { downloadCsv, leadsToCsv } from "../lib/csv";
import {
  autocompleteCity,
  cancelJob,
  cancelScraperSearch,
  checkScraperBinary,
  friendlyError,
  getLeads,
  getProviderCounts,
  getSearchJobs,
  importScraperJson,
  pauseJob,
  pollScraperJob,
  resumeJob,
  searchLeads,
  searchOsm,
  startAdaptiveSearch,
  startScraperSearch,
} from "../lib/api";
import { prefs } from "../lib/prefs";
import { secretStore } from "../lib/secretStore";
import type { Lead, SearchJob } from "../types";

const NICHOS = ["Dentista", "Nutricionista", "Advogado", "Clínica", "Academia", "Salão de beleza", "Restaurante", "Pet shop", "Imobiliária", "Oficina"];

function temp(score: number) {
  if (score >= 60) return { label: "Quente", cls: "bg-green-100 text-green-800" };
  if (score >= 30) return { label: "Morno", cls: "bg-yellow-100 text-yellow-800" };
  return { label: "Frio", cls: "bg-neutral-100 text-neutral-600" };
}

export default function SearchPage() {
  const [query, setQuery] = useState("Dentista");
  const [city, setCity] = useState("Picos, Piauí");
  const [sugestoes, setSugestoes] = useState<{ display_name: string; lat: number; lon: number }[]>([]);
  const [coords, setCoords] = useState<[number, number] | null>(null);
  const [radiusKm, setRadiusKm] = useState(5);
  const [provider, setProvider] = useState<"osm" | "scraper" | "places">("osm");
  const [strategy, setStrategy] = useState<"single" | "adaptive">("adaptive");
  const [scraperEmail, setScraperEmail] = useState(false);
  const [soSemSite, setSoSemSite] = useState(true);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [jobs, setJobs] = useState<SearchJob[]>([]);
  const [leads, setLeads] = useState<Lead[]>([]);
  const [totalRaio, setTotalRaio] = useState<number | null>(null);
  const [scraperMsg, setScraperMsg] = useState<string | null>(null);
  const [binaryMsg, setBinaryMsg] = useState<string | null>(null);
  const [binaryOk, setBinaryOk] = useState<boolean | null>(null);
  const [counts, setCounts] = useState<Record<number, string>>({});
  const [scraperJob, setScraperJob] = useState<number | null>(null);
  const [scraperInfo, setScraperInfo] = useState<string | null>(null);
  const debounce = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pollTimer = useRef<ReturnType<typeof setInterval> | null>(null);

  async function loadJobs() {
    try {
      setJobs(await getSearchJobs());
    } catch {
      /* ignora antes da primeira inicialização */
    }
  }

  useEffect(() => {
    prefs.load().then((p) => {
      if (p.lastQuery) setQuery(p.lastQuery);
      if (p.lastCity) setCity(p.lastCity);
      if (p.lastRadius) setRadiusKm(p.lastRadius);
    });
    loadJobs();
    checkScraperBinary()
      .then((b) => {
        setBinaryOk(b.available);
        setBinaryMsg(b.message);
      })
      .catch(() => setBinaryOk(false));
    return () => {
      if (pollTimer.current) clearInterval(pollTimer.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function onCityChange(v: string) {
    setCity(v);
    setCoords(null);
    if (debounce.current) clearTimeout(debounce.current);
    if (v.trim().length < 3) {
      setSugestoes([]);
      return;
    }
    debounce.current = setTimeout(async () => {
      try {
        setSugestoes(await autocompleteCity(v.trim()));
      } catch {
        setSugestoes([]);
      }
    }, 400);
  }

  async function onSearch() {
    setLoading(true);
    setMessage(null);
    setError(null);
    try {
      await prefs.save({ lastQuery: query, lastCity: city, lastRadius: radiusKm });
      if (provider === "osm") {
        const r = await searchOsm(query, city, radiusKm);
        setMessage(`${r.result_count} encontrados, ${r.new_count} novos no mapa aberto.`);
      } else if (provider === "scraper") {
        const { job_id } = await startScraperSearch(query, city, radiusKm, scraperEmail);
        setScraperJob(job_id);
        setScraperInfo("Busca iniciada…");
        setMessage(null);
        setLoading(false);
        if (pollTimer.current) clearInterval(pollTimer.current);
        pollTimer.current = setInterval(async () => {
          try {
            const p = await pollScraperJob(job_id);
            if (p.running) {
              const mm = Math.floor(p.elapsed_secs / 60);
              const ss = p.elapsed_secs % 60;
              setScraperInfo(`Buscando há ${mm > 0 ? `${mm}min ` : ""}${ss}s…`);
              setMessage(null);
            } else {
              if (pollTimer.current) clearInterval(pollTimer.current);
              setScraperJob(null);
              setScraperInfo(null);
              setMessage(`${p.imported} novos + ${p.merged} unificados (${p.skipped} ignorados). Job #${job_id}.`);
              await loadJobs();
              await loadResults();
            }
          } catch (e) {
            if (pollTimer.current) clearInterval(pollTimer.current);
            setScraperJob(null);
            setScraperInfo(null);
            setError(friendlyError(e));
            await loadJobs();
          }
        }, 4000);
        return;
      } else {
        const apiKey = await secretStore.getApiKey();
        if (!apiKey) {
          setError("Salve sua chave em Configurações primeiro — ou use o Scraper local, que não precisa de chave.");
          return;
        }
        if (strategy === "single") {
          const r = await searchLeads({ query, city, radiusKm, apiKey });
          setMessage(`Busca inicial: ${r.result_count} encontrados, ${r.new_count} novos.`);
        } else {
          const r = await startAdaptiveSearch({ query, city, radiusKm, apiKey });
          setMessage(`Cobertura adaptativa concluída. Job #${r.job_id} — veja abaixo.`);
        }
      }
      await loadJobs();
      await loadResults();
    } catch (e) {
      setError(friendlyError(e));
    } finally {
      setLoading(false);
    }
  }

  async function loadResults() {
    try {
      const todos = await getLeads({ statusFilter: undefined });
      setTotalRaio(todos.length);
      const filtrados = soSemSite ? todos.filter((l) => !l.website) : todos;
      setLeads(filtrados.slice(0, 100));
      const comCoords = filtrados.find((l) => l.latitude != null && l.longitude != null);
      if (!coords && comCoords) setCoords([comCoords.latitude!, comCoords.longitude!]);
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  useEffect(() => {
    loadResults();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [soSemSite]);

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

  function exportar() {
    if (leads.length === 0) return;
    downloadCsv(`leads-${query}-${city}.csv`.replace(/\s+/g, "-"), leadsToCsv(leads));
  }

  const semSite = leads.filter((l) => !l.website).length;

  return (
    <div>
      <h1 className="text-xl font-semibold tracking-tight">Buscar leads</h1>
      <p className="mt-1 text-sm text-neutral-500">Digite a cidade e escolha na lista — a última cidade fica salva.</p>

      <div className="mt-4 grid grid-cols-[320px_1fr] gap-4">
        <Card>
          <div className="grid gap-3">
            <div>
              <Label>Nicho</Label>
              <Input value={query} onChange={(e) => setQuery(e.target.value)} list="nichos" placeholder="Ex: Dentista" />
              <datalist id="nichos">
                {NICHOS.map((n) => <option key={n} value={n} />)}
              </datalist>
            </div>
            <div className="relative">
              <Label>Cidade</Label>
              <Input value={city} onChange={(e) => onCityChange(e.target.value)} placeholder="Ex: Picos, Piauí" />
              {sugestoes.length > 0 && (
                <div className="absolute z-10 mt-1 max-h-44 w-full overflow-auto rounded-lg border border-neutral-200 bg-white shadow-lg">
                  {sugestoes.map((s) => (
                    <button
                      key={s.display_name}
                      className="block w-full px-3 py-2 text-left text-sm hover:bg-neutral-100"
                      onClick={() => {
                        setCity(s.display_name);
                        setCoords([s.lat, s.lon]);
                        setSugestoes([]);
                      }}
                    >
                      {s.display_name}
                    </button>
                  ))}
                </div>
              )}
            </div>
            <div>
              <Label>Raio: {radiusKm} km</Label>
              <input type="range" min={1} max={50} value={radiusKm} onChange={(e) => setRadiusKm(Number(e.target.value))} className="w-full accent-green-600" />
            </div>
            <div>
              <Label>Fonte dos dados</Label>
              <div className="flex flex-wrap gap-2">
                <GhostButton onClick={() => setProvider("osm")} className={provider === "osm" ? "border-green-700 bg-green-700 text-white hover:bg-green-700" : ""}>Mapa aberto · grátis</GhostButton>
                <GhostButton onClick={() => setProvider("scraper")} className={provider === "scraper" ? "border-green-700 bg-green-700 text-white hover:bg-green-700" : ""}>Scraper local</GhostButton>
                <GhostButton onClick={() => setProvider("places")} className={provider === "places" ? "border-neutral-900 bg-neutral-900 text-white hover:bg-neutral-900" : ""}>API Google</GhostButton>
              </div>
              {provider === "osm" && <p className="mt-1 text-xs text-neutral-500">OpenStreetMap: rápido, sem chave e sem instalar nada.</p>}
            </div>
            {provider === "places" && (
              <div>
                <Label>Estratégia</Label>
                <div className="flex gap-2">
                  <GhostButton onClick={() => setStrategy("single")} className={strategy === "single" ? "border-neutral-900 bg-neutral-900 text-white hover:bg-neutral-900" : ""}>Simples</GhostButton>
                  <GhostButton onClick={() => setStrategy("adaptive")} className={strategy === "adaptive" ? "border-neutral-900 bg-neutral-900 text-white hover:bg-neutral-900" : ""}>Adaptativa</GhostButton>
                </div>
              </div>
            )}
            {provider === "scraper" && (
              <label className="flex items-center gap-2 text-sm text-neutral-700">
                <input type="checkbox" checked={scraperEmail} onChange={(e) => setScraperEmail(e.target.checked)} className="accent-green-600" />
                Buscar e-mails nos sites (mais lento)
              </label>
            )}
            <label className="flex items-center gap-2 text-sm text-neutral-700">
              <input type="checkbox" checked={soSemSite} onChange={(e) => setSoSemSite(e.target.checked)} className="accent-green-600" />
              Somente sem site
            </label>
            {binaryOk === false && provider === "scraper" && (
              <p className="rounded-lg bg-yellow-50 px-3 py-2 text-xs text-yellow-800">
                Scraper não encontrado — instale pelo passo “Scraper local” abaixo ou use Importar JSON.
              </p>
            )}
            {scraperJob == null ? (
              <Button onClick={onSearch} disabled={loading} className="bg-green-600 hover:bg-green-500">
                {loading ? "Buscando…" : soSemSite ? "Buscar leads sem site" : "Buscar leads"}
              </Button>
            ) : (
              <div className="grid gap-2">
                <p className="text-sm text-neutral-700">{scraperInfo ?? "Buscando…"}</p>
                <GhostButton
                  onClick={async () => {
                    try {
                      await cancelScraperSearch(scraperJob);
                    } catch {
                      /* já terminou */
                    }
                    if (pollTimer.current) clearInterval(pollTimer.current);
                    setScraperJob(null);
                    setScraperInfo(null);
                    setMessage("Busca cancelada.");
                    loadJobs();
                  }}
                >
                  Cancelar busca
                </GhostButton>
              </div>
            )}
            {message && <p className="text-sm text-neutral-700">{message}</p>}
            {error && <p className="text-sm text-red-600">{error}</p>}
          </div>
        </Card>

        <div className="grid gap-3">
          <LeadMap center={coords} radiusKm={radiusKm} leads={leads} />
          <Card>
            <div className="flex items-center justify-between">
              <p className="text-sm text-neutral-700">
                <b>{semSite}</b> de <b>{leads.length}</b> sem site{totalRaio != null ? ` (${totalRaio} no total)` : ""}
              </p>
              <div className="flex gap-2">
                <GhostButton onClick={exportar} disabled={leads.length === 0}>Exportar CSV</GhostButton>
                <GhostButton onClick={loadResults}>Atualizar</GhostButton>
              </div>
            </div>
            <div className="mt-2 max-h-64 divide-y divide-neutral-100 overflow-auto">
              {leads.slice(0, 30).map((l) => {
                const t = temp(l.score ?? 0);
                return (
                  <div key={l.id} className="flex items-center justify-between gap-2 py-2">
                    <div>
                      <Link to={`/leads/${l.id}`} className="text-sm font-medium hover:underline">{l.canonical_name}</Link>
                      <div className="text-xs text-neutral-500">{l.address ?? "—"}</div>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className={`rounded-full px-2 py-0.5 text-xs ${t.cls}`}>{t.label}</span>
                      <span className="text-xs tabular-nums text-neutral-500">{l.score ?? 0}</span>
                    </div>
                  </div>
                );
              })}
              {leads.length === 0 && <p className="py-6 text-center text-sm text-neutral-400">Nenhum lead — faça uma busca.</p>}
            </div>
          </Card>
        </div>
      </div>

      <h2 className="mt-8 text-sm font-semibold uppercase tracking-wide text-neutral-500">Buscas anteriores</h2>
      <div className="mt-3 grid gap-3">
        {jobs.map((j) => (
          <Card key={j.id}>
            <div className="flex items-center justify-between gap-4">
              <div>
                <Link to={`/jobs/${j.id}`} className="text-sm font-medium hover:underline">#{j.id} {j.query} · {j.city}</Link>
                <div className="mt-1 text-xs text-neutral-500">
                  {j.strategy} · {j.status} · {j.result_count} resultados / {j.new_count} novos ·{" "}
                  {j.total_cells > 0 ? `${j.completed_cells}/${j.total_cells} células · ${Math.round(j.coverage * 100)}%` : "busca única"}
                </div>
                {j.total_cells > 0 && (
                  <div className="mt-2 h-1.5 w-64 overflow-hidden rounded-full bg-neutral-200">
                    <div className="h-full bg-green-600" style={{ width: `${Math.round(j.coverage * 100)}%` }} />
                  </div>
                )}
                <button
                  className="mt-1 text-xs text-neutral-400 hover:underline"
                  onClick={async () => {
                    try {
                      const c = await getProviderCounts(j.id);
                      setCounts((p) => ({ ...p, [j.id]: c.map((x) => `${x.provider}: ${x.leads}`).join(" · ") || "sem provedores" }));
                    } catch (err) {
                      setCounts((p) => ({ ...p, [j.id]: friendlyError(err) }));
                    }
                  }}
                >
                  {counts[j.id] ?? "Ver provedores"}
                </button>
              </div>
              <div className="flex gap-1">
                {(j.status === "running" || j.status === "pending") && <GhostButton onClick={() => doPause(j.id)}>Pausar</GhostButton>}
                {(j.status === "paused" || j.status === "interrupted" || j.status === "failed") && <GhostButton onClick={() => doResume(j.id)}>Continuar</GhostButton>}
                {j.status !== "cancelled" && j.status !== "completed" && <GhostButton onClick={() => doCancel(j.id)}>Cancelar</GhostButton>}
              </div>
            </div>
          </Card>
        ))}
        {jobs.length === 0 && <p className="text-sm text-neutral-400">Nenhuma busca ainda.</p>}
      </div>

      <h2 className="mt-8 text-sm font-semibold uppercase tracking-wide text-neutral-500">Scraper local — instalação</h2>
      <Card>
        <p className="text-xs text-neutral-500">
          Para buscar sem chave de API, baixe o <b>google-maps-scraper</b> em github.com/gosom/google-maps-scraper/releases
          (Windows: o .exe) e deixe no PATH. Sem ele, use Importar JSON abaixo com um arquivo gerado pelo programa.
        </p>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <GhostButton onClick={async () => {
            const b = await checkScraperBinary();
            setBinaryMsg(`${b.available ? "✓" : "—"} ${b.message}`);
          }}>Verificar programa</GhostButton>
          <label className="cursor-pointer rounded-lg border border-neutral-200 bg-white px-3 py-1.5 text-sm text-neutral-700 hover:bg-neutral-100">
            Importar JSON
            <input type="file" accept=".json,application/json" className="hidden" onChange={async (e) => {
              const f = e.target.files?.[0];
              if (!f) return;
              const text = await f.text();
              try {
                const r = await importScraperJson(query, city, text);
                setScraperMsg(`Importados ${r.imported} novos, ${r.merged} unificados, ${r.skipped} ignorados. Job #${r.job_id}.`);
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
    </div>
  );
}
