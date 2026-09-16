import { useEffect, useState } from "react";
import { Button, Card, GhostButton, Input, Label } from "../components/ui";
import { friendlyError, getScoreConfig, rescoreLeads, updateScoreConfig } from "../lib/api";
import { secretStore } from "../lib/secretStore";
import type { ScoreConfig } from "../types";

export default function SettingsPage() {
  const [key, setKey] = useState("");
  const [saved, setSaved] = useState(false);
  const [hasKey, setHasKey] = useState(false);
  const [cfg, setCfg] = useState<ScoreConfig | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    secretStore.getApiKey().then((k) => setHasKey(!!k));
    getScoreConfig().then(setCfg).catch(() => {});
  }, []);

  async function save() {
    await secretStore.setApiKey(key);
    setKey("");
    setSaved(true);
    setHasKey(true);
    setTimeout(() => setSaved(false), 2500);
  }

  async function saveScore() {
    if (!cfg) return;
    try {
      setErr(null);
      await updateScoreConfig(cfg);
      const r = await rescoreLeads();
      setMsg(`Weights saved. Rescored ${r.rescored} leads.`);
    } catch (e) {
      setErr(friendlyError(e));
    }
  }

  function num<K extends keyof ScoreConfig>(k: K, v: string) {
    if (!cfg) return;
    setCfg({ ...cfg, [k]: Number(v) });
  }

  return (
    <div>
      <h1 className="text-xl font-semibold tracking-tight">Settings</h1>
      <p className="mt-1 text-sm text-neutral-500">Key via SecretStore · scoring weights persisted locally.</p>
      <div className="mt-6 grid max-w-2xl gap-4">
        <Card>
          <Label>Google Places API key</Label>
          <Input type="password" placeholder={hasKey ? "•••••••• (saved)" : "AIza…"} value={key} onChange={(e) => setKey(e.target.value)} />
          <div className="mt-3">
            <Button onClick={save} disabled={!key.trim()}>Save key</Button>
          </div>
          {saved && <p className="mt-2 text-sm text-neutral-600">Saved locally.</p>}
        </Card>
        <Card>
          <div className="text-sm font-medium">Lead score weights</div>
          <p className="mt-1 text-xs text-neutral-500">Operational priority only, not absolute truth.</p>
          {cfg && (
            <div className="mt-3 grid grid-cols-3 gap-2">
              {(["no_website", "website_offline", "has_instagram", "has_phone", "has_email", "rating_gte", "reviews_gte", "active_bonus"] as const).map((k) => (
                <div key={k}>
                  <Label>{k}</Label>
                  <Input type="number" value={cfg[k]} onChange={(e) => num(k, e.target.value)} />
                </div>
              ))}
              <div>
                <Label>rating_threshold</Label>
                <Input type="number" step="0.1" value={cfg.rating_threshold} onChange={(e) => num("rating_threshold", e.target.value)} />
              </div>
              <div>
                <Label>reviews_threshold</Label>
                <Input type="number" value={cfg.reviews_threshold} onChange={(e) => num("reviews_threshold", e.target.value)} />
              </div>
            </div>
          )}
          <div className="mt-3 flex gap-2">
            <GhostButton onClick={saveScore}>Save + rescore</GhostButton>
          </div>
          {msg && <p className="mt-2 text-sm text-neutral-600">{msg}</p>}
          {err && <p className="mt-2 text-sm text-red-600">{err}</p>}
        </Card>
      </div>
    </div>
  );
}
