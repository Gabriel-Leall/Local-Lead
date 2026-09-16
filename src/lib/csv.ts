import type { Lead } from "../types";

const COLS: (keyof Lead)[] = [
  "canonical_name",
  "category",
  "address",
  "phone",
  "website",
  "email",
  "instagram",
  "rating",
  "review_count",
  "score",
  "lead_status",
];

function esc(v: unknown): string {
  const s = v === null || v === undefined ? "" : String(v);
  return /[;"\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

export function leadsToCsv(leads: Lead[]): string {
  const header = ["nome", "categoria", "endereco", "telefone", "site", "email", "instagram", "avaliacao", "avaliacoes", "score", "status"];
  const lines = leads.map((l) => COLS.map((c) => esc(l[c])).join(";"));
  return `﻿${header.join(";")}\n${lines.join("\n")}`;
}

export function downloadCsv(filename: string, csv: string) {
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
