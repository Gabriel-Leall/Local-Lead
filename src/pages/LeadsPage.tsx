import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Button, Card, GhostButton, Input } from "../components/ui";
import {
  bulkSetStatus,
  deleteSavedFilter,
  enrichLeads,
  enrichWebsites,
  friendlyError,
  getLeads,
  getQualificationQueue,
  listSavedFilters,
  rescoreLeads,
  saveNamedFilter,
  updateLeadStatus,
} from "../lib/api";
import { secretStore } from "../lib/secretStore";
import type { Lead, SavedFilter } from "../types";

function cell(v: string | number | null | undefined) {
  if (v === null || v === undefined || v === "") return "—";
  return String(v);
}

function scoreBadge(score: number) {
  const cls = score >= 60 ? "bg-neutral-900 text-white" : score >= 30 ? "bg-neutral-200 text-neutral-800" : "bg-neutral-100 text-neutral-500";
  return <span className={`rounded-full px-2 py-0.5 text-xs tabular-nums ${cls}`}>{score}</span>;
}

export default function LeadsPage() {
  const [leads, setLeads] = useState<Lead[]>([]);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [name, setName] = useState("");
  const [category, setCategory] = useState("");
  const [status, setStatusFilter] = useState("");
  const [presence, setPresence] = useState("");
  const [siteStatus, setSiteStatus] = useState("");
  const [minRating, setMinRating] = useState("");
  const [minReviews, setMinReviews] = useState("");
  const [minScore, setMinScore] = useState("");
  const [queueMode, setQueueMode] = useState(false);
  const [saved, setSaved] = useState<SavedFilter[]>([]);
  const [filterName, setFilterName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  function currentFilterJson() {
    return JSON.stringify({ name, category, status, presence, siteStatus, minRating, minReviews, minScore });
  }

  async function load() {
    try {
      setError(null);
      if (queueMode) {
        setLeads(await getQualificationQueue(50));
        return;
      }
      const hasWebsite = presence === "with-site" ? true : presence === "no-site" ? false : null;
      const hasPhone = presence === "with-phone" ? true : presence === "no-phone" ? false : null;
      const hasEmail = presence === "with-email" ? true : presence === "no-email" ? false : null;
      const hasInstagram = presence === "with-ig" ? true : presence === "no-ig" ? false : null;
      setLeads(
        await getLeads({
          nameFilter: name || undefined,
          categoryFilter: category || undefined,
          statusFilter: status || undefined,
          hasWebsite,
          hasPhone,
          hasEmail,
          hasInstagram,
          websiteStatus: siteStatus || null,
          minRating: minRating ? Number(minRating) : null,
          minReviews: minReviews ? Number(minReviews) : null,
          minScore: minScore ? Number(minScore) : null,
          orderByScore: true,
        })
      );
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  async function loadSaved() {
    try {
      setSaved(await listSavedFilters());
    } catch {
      /* ignore */
    }
  }

  useEffect(() => {
    load();
    loadSaved();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [queueMode]);

  function toggle(id: number) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelected(next);
  }

  async function changeStatus(id: number, s: string) {
    await updateLeadStatus(id, s);
    load();
  }

  async function onBulk(s: string) {
    if (selected.size === 0) return;
    await bulkSetStatus([...selected], s);
    setSelected(new Set());
    load();
  }

  async function onRescore() {
    setBusy(true);
    try {
      const r = await rescoreLeads();
      setInfo(`Rescored ${r.rescored} leads.`);
      load();
    } catch (e) {
      setError(friendlyError(e));
    } finally {
      setBusy(false);
    }
  }

  async function onEnrichDetails() {
    setBusy(true);
    setInfo(null);
    setError(null);
    try {
      const apiKey = await secretStore.getApiKey();
      if (!apiKey) {
        setError("Save your Google Places API key in Settings first.");
        return;
      }
      const ids = selected.size > 0 ? [...selected] : leads.slice(0, 20).map((l) => l.id);
      if (ids.length === 0) {
        setError("No leads to enrich.");
        return;
      }
      const r = await enrichLeads(ids, apiKey);
      setInfo(`Details: enriched ${r.enriched}, failed ${r.failed}.`);
      setSelected(new Set());
      load();
    } catch (e) {
      setError(friendlyError(e));
    } finally {
      setBusy(false);
    }
  }

  async function onEnrichWebsites() {
    setBusy(true);
    setInfo(null);
    setError(null);
    try {
      const ids = selected.size > 0 ? [...selected] : leads.filter((l) => l.website).slice(0, 20).map((l) => l.id);
      if (ids.length === 0) {
        setError("No leads with website to crawl.");
        return;
      }
      const r = await enrichWebsites(ids);
      setInfo(`Websites: crawled ${r.enriched}, failed ${r.failed}.`);
      setSelected(new Set());
      load();
    } catch (e) {
      setError(friendlyError(e));
    } finally {
      setBusy(false);
    }
  }

  async function onSaveFilter() {
    if (!filterName.trim()) return;
    await saveNamedFilter(filterName.trim(), currentFilterJson());
    setFilterName("");
    loadSaved();
  }

  function applySaved(f: SavedFilter) {
    try {
      const j = JSON.parse(f.filters_json);
      setName(j.name || "");
      setCategory(j.category || "");
      setStatusFilter(j.status || "");
      setPresence(j.presence || "");
      setSiteStatus(j.siteStatus || "");
      setMinRating(j.minRating || "");
      setMinReviews(j.minReviews || "");
      setMinScore(j.minScore || "");
      setQueueMode(false);
      setTimeout(load, 50);
    } catch {
      setError("Saved filter is invalid.");
    }
  }

  return (
    <div>
      <div className="flex items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold tracking-tight">Leads</h1>
          <p className="mt-1 text-sm text-neutral-500">
            {queueMode ? "Qualification queue: top new leads by score." : "Score prioritizes no-website + contacts + reviews."}
          </p>
        </div>
        <div className="flex gap-2">
          <GhostButton onClick={() => setQueueMode(!queueMode)}>{queueMode ? "Exit queue" : "Queue mode"}</GhostButton>
          <GhostButton onClick={onRescore} disabled={busy}>Rescore</GhostButton>
          <GhostButton onClick={onEnrichDetails} disabled={busy || leads.length === 0}>Details</GhostButton>
          <Button onClick={onEnrichWebsites} disabled={busy || leads.length === 0}>Websites</Button>
        </div>
      </div>

      {!queueMode && (
        <div className="mt-4">
          <Card>
            <div className="grid grid-cols-4 gap-2">
              <Input placeholder="Search by name" value={name} onChange={(e) => setName(e.target.value)} />
              <Input placeholder="Category" value={category} onChange={(e) => setCategory(e.target.value)} />
              <select value={status} onChange={(e) => setStatusFilter(e.target.value)} className="rounded-lg border border-neutral-200 bg-white px-3 py-2 text-sm">
                <option value="">All statuses</option>
                {["new","qualified","message_ready","contacted","replied","interested","meeting","won","lost","skipped","do_not_contact"].map((s) => (
                  <option key={s} value={s}>{s}</option>
                ))}
              </select>
              <select value={presence} onChange={(e) => setPresence(e.target.value)} className="rounded-lg border border-neutral-200 bg-white px-3 py-2 text-sm">
                <option value="">All presence</option>
                <option value="with-site">With website</option>
                <option value="no-site">No website</option>
                <option value="with-phone">With phone</option>
                <option value="with-email">With email</option>
                <option value="with-ig">With Instagram</option>
              </select>
              <Input placeholder="Rating ≥" value={minRating} onChange={(e) => setMinRating(e.target.value)} />
              <Input placeholder="Reviews ≥" value={minReviews} onChange={(e) => setMinReviews(e.target.value)} />
              <Input placeholder="Score ≥" value={minScore} onChange={(e) => setMinScore(e.target.value)} />
              <select value={siteStatus} onChange={(e) => setSiteStatus(e.target.value)} className="rounded-lg border border-neutral-200 bg-white px-3 py-2 text-sm">
                <option value="">Site: all</option>
                <option value="active">active</option>
                <option value="unreachable">unreachable</option>
                <option value="redirect">redirect</option>
                <option value="parked">parked</option>
                <option value="none">none</option>
              </select>
              <div className="col-span-4 flex flex-wrap items-center gap-2">
                <GhostButton onClick={load}>Filter</GhostButton>
                <GhostButton onClick={() => { setName(""); setCategory(""); setStatusFilter(""); setPresence(""); setSiteStatus(""); setMinRating(""); setMinReviews(""); setMinScore(""); }}>Clear</GhostButton>
                {selected.size > 0 && (
                  <>
                    <GhostButton onClick={() => onBulk("qualified")}>Qualify ({selected.size})</GhostButton>
                    <GhostButton onClick={() => onBulk("skipped")}>Skip ({selected.size})</GhostButton>
                  </>
                )}
              </div>
              <div className="col-span-4 flex flex-wrap items-center gap-2 border-t border-neutral-100 pt-2">
                <Input placeholder="Save current as…" value={filterName} onChange={(e) => setFilterName(e.target.value)} className="max-w-48" />
                <GhostButton onClick={onSaveFilter}>Save filter</GhostButton>
                {saved.map((f) => (
                  <span key={f.id} className="flex items-center gap-1 rounded-full bg-neutral-100 py-1 pl-3 pr-1 text-xs">
                    <button onClick={() => applySaved(f)} className="hover:underline">{f.name}</button>
                    <button onClick={() => deleteSavedFilter(f.id).then(loadSaved)} className="rounded-full px-1 text-neutral-400 hover:text-neutral-800">×</button>
                  </span>
                ))}
              </div>
            </div>
          </Card>
        </div>
      )}

      {info && <p className="mt-3 text-sm text-neutral-700">{info}</p>}
      {error && <p className="mt-3 text-sm text-red-600">{error}</p>}
      <div className="mt-4 overflow-x-auto rounded-xl border border-neutral-200 bg-white">
        <table className="w-full min-w-[960px] text-left text-sm">
          <thead className="border-b border-neutral-200 bg-neutral-50 text-xs uppercase tracking-wide text-neutral-500">
            <tr>
              <th className="px-3 py-2"><input type="checkbox" checked={selected.size > 0 && selected.size === leads.length} onChange={() => setSelected(selected.size === leads.length ? new Set() : new Set(leads.map((l) => l.id)))} /></th>
              <th className="px-4 py-2">Score</th>
              <th className="px-4 py-2">Business</th>
              <th className="px-4 py-2">Site</th>
              <th className="px-4 py-2">IG</th>
              <th className="px-4 py-2">Email</th>
              <th className="px-4 py-2">Rating</th>
              <th className="px-4 py-2">Status</th>
              <th className="px-4 py-2" />
            </tr>
          </thead>
          <tbody>
            {leads.map((l) => (
              <tr key={l.id} className="border-b border-neutral-100 last:border-0 hover:bg-neutral-50">
                <td className="px-3 py-2"><input type="checkbox" checked={selected.has(l.id)} onChange={() => toggle(l.id)} /></td>
                <td className="px-4 py-2" title={l.score_reasons ?? ""}>{scoreBadge(l.score ?? 0)}</td>
                <td className="px-4 py-2"><Link to={`/leads/${l.id}`} className="font-medium hover:underline">{l.canonical_name}</Link><div className="text-xs text-neutral-500">{cell(l.website_status)}{l.phone ? ` · ${l.phone}` : ""}</div></td>
                <td className="px-4 py-2">{l.website ? "✓" : "—"}</td>
                <td className="px-4 py-2">{l.instagram ? `@${l.instagram}` : "—"}</td>
                <td className="px-4 py-2">{cell(l.email)}</td>
                <td className="px-4 py-2 tabular-nums">{l.rating ?? "—"}</td>
                <td className="px-4 py-2"><span className="rounded-full bg-neutral-100 px-2 py-0.5 text-xs">{l.lead_status}</span></td>
                <td className="px-4 py-2 text-right">
                  <div className="flex justify-end gap-1">
                    <GhostButton onClick={() => changeStatus(l.id, "qualified")}>Qualify</GhostButton>
                    <GhostButton onClick={() => changeStatus(l.id, "skipped")}>Skip</GhostButton>
                  </div>
                </td>
              </tr>
            ))}
            {leads.length === 0 && (
              <tr><td colSpan={9} className="px-4 py-8 text-center text-neutral-400">No leads in queue. Enrich more or clear filters.</td></tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
