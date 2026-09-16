import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { Button, Card, GhostButton } from "../components/ui";
import {
  addNote,
  friendlyError,
  generateMessage,
  getActivity,
  getLeadDetail,
  getLeadMessages,
  listTemplates,
  previewMessage,
  setOutreachStatus,
  setPipelineStatus,
} from "../lib/api";
import type { Lead, LeadNote, MessageTemplate, OutreachMessage, StatusEvent } from "../types";
import { PIPELINE } from "../types";

export default function LeadDetailPage() {
  const { id } = useParams();
  const leadId = Number(id);
  const [lead, setLead] = useState<Lead | null>(null);
  const [templates, setTemplates] = useState<MessageTemplate[]>([]);
  const [tpl, setTpl] = useState("no_website");
  const [preview, setPreview] = useState<{ subject: string; message: string } | null>(null);
  const [msgs, setMsgs] = useState<OutreachMessage[]>([]);
  const [history, setHistory] = useState<StatusEvent[]>([]);
  const [notes, setNotes] = useState<LeadNote[]>([]);
  const [noteText, setNoteText] = useState("");
  const [followUp, setFollowUp] = useState("");
  const [info, setInfo] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    try {
      setLead(await getLeadDetail(leadId));
      setMsgs(await getLeadMessages(leadId));
      const act = await getActivity(leadId);
      setHistory(act.history);
      setNotes(act.notes);
      if (templates.length === 0) {
        const t = await listTemplates();
        setTemplates(t);
      }
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [leadId]);

  async function onPreview() {
    try {
      setPreview(await previewMessage(leadId, tpl));
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  async function onGenerate() {
    try {
      await generateMessage(leadId, tpl);
      setInfo("Rascunho gerado — revise antes de contatar.");
      setPreview(null);
      load();
    } catch (e) {
      setError(friendlyError(e));
    }
  }

  async function onStatus(msgId: number, status: string) {
    await setOutreachStatus(msgId, status);
    load();
  }

  function copy(text: string) {
    navigator.clipboard.writeText(text);
    setInfo("Copiado.");
  }

  if (!lead) return <p className="text-sm text-neutral-500">Carregando…</p>;

  const emailHref = lead.email ? `mailto:${lead.email}?subject=${encodeURIComponent(preview?.subject ?? msgs[0]?.subject ?? "")}` : null;

  return (
    <div>
      <h1 className="text-xl font-semibold tracking-tight">{lead.canonical_name}</h1>
      <p className="mt-1 text-sm text-neutral-500">
        {lead.category ?? "—"} · {lead.address ?? "—"} · score {lead.score ?? 0} · {lead.lead_status}
      </p>

      <div className="mt-4 grid grid-cols-2 gap-4">
        <Card>
          <div className="text-xs uppercase tracking-wide text-neutral-500">Contato</div>
          <dl className="mt-2 space-y-1 text-sm">
            <div className="flex justify-between"><dt className="text-neutral-500">Telefone</dt><dd>{lead.phone ?? "—"}</dd></div>
            <div className="flex justify-between"><dt className="text-neutral-500">E-mail</dt><dd>{lead.email ?? "—"}</dd></div>
            <div className="flex justify-between"><dt className="text-neutral-500">Instagram</dt><dd>{lead.instagram ? `@${lead.instagram}` : "—"}</dd></div>
            <div className="flex justify-between"><dt className="text-neutral-500">Site</dt><dd>{lead.website ?? "—"} ({lead.website_status ?? "desconhecido"})</dd></div>
            <div className="flex justify-between"><dt className="text-neutral-500">Nota</dt><dd>{lead.rating ?? "—"} ({lead.review_count ?? "—"})</dd></div>
          </dl>
          <div className="mt-3 flex flex-wrap gap-1">
            {lead.website && <a href={lead.website.startsWith("http") ? lead.website : `https://${lead.website}`} target="_blank" rel="noreferrer"><GhostButton>Abrir site</GhostButton></a>}
            {lead.instagram && <a href={`https://instagram.com/${lead.instagram}`} target="_blank" rel="noreferrer"><GhostButton>Abrir IG</GhostButton></a>}
            {emailHref && <a href={emailHref}><GhostButton>Abrir e-mail</GhostButton></a>}
            {lead.whatsapp && <span className="text-xs text-neutral-500">WA: {lead.whatsapp}</span>}
          </div>
          <div className="mt-3">
            <div className="text-xs uppercase tracking-wide text-neutral-500">Pipeline</div>
            <div className="mt-1 flex flex-wrap gap-1">
              {PIPELINE.map((s) => (
                <button
                  key={s}
                  onClick={() => setPipelineStatus(lead.id, s).then(load)}
                  className={`rounded-full px-2 py-0.5 text-xs ${lead.lead_status === s ? "bg-neutral-900 text-white" : "bg-neutral-100 text-neutral-600 hover:bg-neutral-200"}`}
                >
                  {s}
                </button>
              ))}
            </div>
            {lead.follow_up_at && <p className="mt-2 text-xs text-neutral-500">Retorno em: {lead.follow_up_at}</p>}
          </div>
        </Card>

        <Card>
          <div className="text-xs uppercase tracking-wide text-neutral-500">Nova mensagem</div>
          <select value={tpl} onChange={(e) => setTpl(e.target.value)} className="mt-2 w-full rounded-lg border border-neutral-200 bg-white px-3 py-2 text-sm">
            {templates.map((t) => <option key={t.id} value={t.id}>{t.label}</option>)}
          </select>
          <div className="mt-2 flex gap-2">
            <GhostButton onClick={onPreview}>Prévia</GhostButton>
            <Button onClick={onGenerate}>Gerar rascunho</Button>
          </div>
          {preview && (
            <div className="mt-3 rounded-lg bg-neutral-50 p-3 text-sm">
              <div className="font-medium">{preview.subject}</div>
              <pre className="mt-1 whitespace-pre-wrap font-sans">{preview.message}</pre>
              <div className="mt-2"><GhostButton onClick={() => copy(`${preview.subject}\n\n${preview.message}`)}>Copiar</GhostButton></div>
            </div>
          )}
        </Card>
      </div>

      {info && <p className="mt-3 text-sm text-neutral-700">{info}</p>}
      {error && <p className="mt-3 text-sm text-red-600">{error}</p>}

      <h2 className="mt-6 text-sm font-semibold uppercase tracking-wide text-neutral-500">Notas e retorno</h2>
      <Card>
        <div className="flex gap-2">
          <input value={noteText} onChange={(e) => setNoteText(e.target.value)} placeholder="Adicionar nota…" className="w-full rounded-lg border border-neutral-200 px-3 py-2 text-sm outline-none focus:border-neutral-900" />
          <input type="date" value={followUp} onChange={(e) => setFollowUp(e.target.value)} className="rounded-lg border border-neutral-200 px-3 py-2 text-sm" />
          <GhostButton onClick={async () => {
            if (!noteText.trim()) return;
            await addNote(lead.id, noteText.trim(), followUp || null);
            setNoteText("");
            setFollowUp("");
            load();
          }}>Salvar</GhostButton>
        </div>
        <div className="mt-2 grid gap-1">
          {notes.map((n) => (
            <div key={n.id} className="text-sm"><span className="text-neutral-500">{n.created_at.slice(0, 10)} · </span>{n.note}{n.follow_up_at && <span className="text-neutral-500"> → retorno {n.follow_up_at}</span>}</div>
          ))}
          {notes.length === 0 && <p className="text-sm text-neutral-400">Sem notas ainda.</p>}
        </div>
      </Card>

      <h2 className="mt-6 text-sm font-semibold uppercase tracking-wide text-neutral-500">Atividade</h2>
      <Card>
        <div className="grid gap-1">
          {history.map((h) => (
            <div key={h.id} className="text-sm"><span className="text-neutral-500">{h.created_at.slice(0, 16).replace("T", " ")} · </span>{h.from_status ?? "—"} → <b>{h.to_status}</b>{h.note ? ` — ${h.note}` : ""}</div>
          ))}
          {history.length === 0 && <p className="text-sm text-neutral-400">Sem mudanças de status ainda.</p>}
        </div>
      </Card>

      <h2 className="mt-6 text-sm font-semibold uppercase tracking-wide text-neutral-500">Rascunhos ({msgs.length})</h2>
      <div className="mt-2 grid gap-2">
        {msgs.map((m) => (
          <Card key={m.id}>
            <div className="flex items-center justify-between">
              <div className="text-sm font-medium">{m.subject}</div>
              <span className="rounded-full bg-neutral-100 px-2 py-0.5 text-xs">{m.status}</span>
            </div>
            <pre className="mt-2 whitespace-pre-wrap font-sans text-sm text-neutral-700">{m.message}</pre>
            <div className="mt-2 flex flex-wrap gap-1">
              <GhostButton onClick={() => copy(`${m.subject ?? ""}\n\n${m.message}`)}>Copiar</GhostButton>
              {m.status === "generated" && <GhostButton onClick={() => onStatus(m.id, "approved")}>Aprovar</GhostButton>}
              {m.status === "approved" && <GhostButton onClick={() => onStatus(m.id, "sent")}>Marcar enviado</GhostButton>}
              {m.status === "sent" && <GhostButton onClick={() => onStatus(m.id, "replied")}>Marcar respondido</GhostButton>}
              {!["sent", "replied", "skipped"].includes(m.status) && <GhostButton onClick={() => onStatus(m.id, "skipped")}>Pular</GhostButton>}
            </div>
          </Card>
        ))}
        {msgs.length === 0 && <p className="text-sm text-neutral-400">Sem rascunhos — veja a prévia e gere um.</p>}
      </div>
    </div>
  );
}
