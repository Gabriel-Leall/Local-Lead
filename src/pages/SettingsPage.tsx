import { useEffect, useState } from "react";
import { Button, Card, GhostButton, Input, Label } from "../components/ui";
import { friendlyError, getScoreConfig, rescoreLeads, testPlacesKey, updateScoreConfig } from "../lib/api";
import { secretStore } from "../lib/secretStore";
import type { ScoreConfig } from "../types";

const LABELS: Record<string, string> = {
  no_website: "Sem site",
  website_offline: "Site fora do ar",
  has_instagram: "Tem Instagram",
  has_phone: "Tem telefone",
  has_email: "Tem e-mail",
  rating_gte: "Boa avaliação",
  reviews_gte: "Muitas avaliações",
  active_bonus: "Negócio ativo",
};

export default function SettingsPage() {
  const [key, setKey] = useState("");
  const [saved, setSaved] = useState(false);
  const [hasKey, setHasKey] = useState(false);
  const [cfg, setCfg] = useState<ScoreConfig | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);

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

  async function onTest() {
    setTesting(true);
    setMsg(null);
    setErr(null);
    try {
      const k = key.trim() || (await secretStore.getApiKey());
      if (!k) {
        setErr("Digite a chave ou salve antes de testar.");
        return;
      }
      const r = await testPlacesKey(k);
      setMsg(`Chave funcionando! Encontrados ${r.found} resultados no teste (São Paulo).`);
    } catch (e) {
      setErr(friendlyError(e));
    } finally {
      setTesting(false);
    }
  }

  async function saveScore() {
    if (!cfg) return;
    try {
      setErr(null);
      await updateScoreConfig(cfg);
      const r = await rescoreLeads();
      setMsg(`Pesos salvos. ${r.rescored} leads recalculados.`);
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
      <h1 className="text-xl font-semibold tracking-tight">Configurações</h1>
      <p className="mt-1 text-sm text-neutral-500">Chave guardada localmente · nunca registrada em log.</p>
      <div className="mt-6 grid max-w-2xl gap-4">
        <Card>
          <Label>Chave da API do Google Places</Label>
          <Input type="password" placeholder={hasKey ? "•••••••• (salva)" : "AIza…"} value={key} onChange={(e) => setKey(e.target.value)} />
          <p className="mt-1 text-xs text-neutral-500">
            Ative a <b>Places API (New)</b> no Google Cloud e o faturamento, senão a busca falha.
          </p>
          <div className="mt-3 flex gap-2">
            <Button onClick={save} disabled={!key.trim()}>Salvar chave</Button>
            <GhostButton onClick={onTest} disabled={testing}>{testing ? "Testando…" : "Testar chave"}</GhostButton>
          </div>
          {saved && <p className="mt-2 text-sm text-neutral-600">Salva localmente.</p>}
          {msg && <p className="mt-2 text-sm text-green-700">{msg}</p>}
          {err && <p className="mt-2 text-sm text-red-600">{err}</p>}
        </Card>
        <Card>
          <div className="text-sm font-medium">Pesos do score</div>
          <p className="mt-1 text-xs text-neutral-500">Prioridade operacional, não verdade absoluta.</p>
          {cfg && (
            <div className="mt-3 grid grid-cols-3 gap-2">
              {(Object.keys(LABELS) as (keyof ScoreConfig)[]).map((k) => (
                <div key={k}>
                  <Label>{LABELS[k]}</Label>
                  <Input type="number" value={cfg[k] as number} onChange={(e) => num(k, e.target.value)} />
                </div>
              ))}
              <div>
                <Label>Nota mínima</Label>
                <Input type="number" step="0.1" value={cfg.rating_threshold} onChange={(e) => num("rating_threshold", e.target.value)} />
              </div>
              <div>
                <Label>Mín. avaliações</Label>
                <Input type="number" value={cfg.reviews_threshold} onChange={(e) => num("reviews_threshold", e.target.value)} />
              </div>
            </div>
          )}
          <div className="mt-3 flex gap-2">
            <GhostButton onClick={saveScore}>Salvar + recalcular</GhostButton>
          </div>
        </Card>
      </div>
    </div>
  );
}
