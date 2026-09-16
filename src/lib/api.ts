import { invoke } from "@tauri-apps/api/core";
import type { DashboardStats, Lead, LeadNote, MessageTemplate, OutreachMessage, ProviderCount, SavedFilter, ScoreConfig, SearchCell, SearchJob, SearchResult, StatusEvent } from "../types";

export async function searchLeads(args: {
  query: string;
  city: string;
  radiusKm: number;
  apiKey: string;
}): Promise<SearchResult> {
  return invoke<SearchResult>("search_leads", {
    query: args.query,
    city: args.city,
    radiusKm: args.radiusKm,
    apiKey: args.apiKey,
  });
}

export async function startAdaptiveSearch(args: {
  query: string;
  city: string;
  radiusKm: number;
  apiKey: string;
}): Promise<{ job_id: number }> {
  return invoke("start_adaptive_search_cmd", {
    query: args.query,
    city: args.city,
    radiusKm: args.radiusKm,
    apiKey: args.apiKey,
  });
}

export async function getSearchJobs(): Promise<SearchJob[]> {
  return invoke("get_search_jobs");
}

export async function getSearchCells(jobId: number): Promise<SearchCell[]> {
  return invoke("get_search_cells", { jobId });
}

export async function pauseJob(jobId: number) {
  return invoke("pause_search_job", { jobId });
}

export async function resumeJob(jobId: number, apiKey: string) {
  return invoke("resume_search_job", { jobId, apiKey });
}

export async function cancelJob(jobId: number) {
  return invoke("cancel_search_job", { jobId });
}

export type LeadFilters = {
  nameFilter?: string;
  categoryFilter?: string;
  statusFilter?: string;
  hasWebsite?: boolean | null;
  hasPhone?: boolean | null;
  hasEmail?: boolean | null;
  hasInstagram?: boolean | null;
  websiteStatus?: string | null;
  minRating?: number | null;
  minReviews?: number | null;
  minScore?: number | null;
  orderByScore?: boolean;
};

export async function getLeads(filters: LeadFilters): Promise<Lead[]> {
  return invoke<Lead[]>("get_leads_filtered", {
    nameFilter: filters.nameFilter || null,
    categoryFilter: filters.categoryFilter || null,
    statusFilter: filters.statusFilter || null,
    hasWebsite: filters.hasWebsite ?? null,
    hasPhone: filters.hasPhone ?? null,
    hasEmail: filters.hasEmail ?? null,
    hasInstagram: filters.hasInstagram ?? null,
    websiteStatus: filters.websiteStatus ?? null,
    minRating: filters.minRating ?? null,
    minReviews: filters.minReviews ?? null,
    minScore: filters.minScore ?? null,
    orderByScore: filters.orderByScore ?? false,
    limit: 300,
  });
}

export async function getScoreConfig(): Promise<ScoreConfig> {
  return invoke("get_score_config");
}

export async function updateScoreConfig(config: ScoreConfig): Promise<ScoreConfig> {
  return invoke("update_score_config", { config });
}

export async function rescoreLeads(): Promise<{ rescored: number }> {
  return invoke("rescore_leads");
}

export async function getQualificationQueue(limit = 50): Promise<Lead[]> {
  return invoke("get_qualification_queue", { limit });
}

export async function bulkSetStatus(leadIds: number[], status: string): Promise<number> {
  return invoke("bulk_set_status", { leadIds, status });
}

export async function listSavedFilters(): Promise<SavedFilter[]> {
  return invoke("list_filters");
}

export async function saveNamedFilter(name: string, filtersJson: string): Promise<number> {
  return invoke("save_named_filter", { name, filtersJson });
}

export async function deleteSavedFilter(id: number) {
  return invoke("delete_filter", { id });
}

export async function checkScraperBinary(): Promise<{ available: boolean; message: string }> {
  return invoke("check_scraper_binary");
}

export async function importScraperJson(query: string, city: string, resultsJson: string): Promise<{ job_id: number; imported: number; merged: number; skipped: number }> {
  return invoke("import_scraper_json", { query, city, resultsJson });
}

export async function getProviderCounts(jobId: number): Promise<ProviderCount[]> {
  return invoke("get_provider_counts", { jobId });
}

export async function getActivity(leadId: number): Promise<{ history: StatusEvent[]; notes: LeadNote[] }> {
  return invoke("get_activity", { leadId });
}

export async function addNote(leadId: number, note: string, followUpAt?: string | null): Promise<number> {
  return invoke("add_note", { leadId, note, followUpAt: followUpAt ?? null });
}

export async function setPipelineStatus(leadId: number, status: string, note?: string | null): Promise<void> {
  return invoke("set_pipeline_status", { leadId, status, note: note ?? null });
}

export async function listTemplates(): Promise<MessageTemplate[]> {
  return invoke("list_templates");
}

export async function previewMessage(leadId: number, templateId: string): Promise<{ subject: string; message: string }> {
  return invoke("preview_message", { leadId, templateId });
}

export async function generateMessage(leadId: number, templateId: string, channel?: string): Promise<number> {
  return invoke("generate_message", { leadId, templateId, channel: channel ?? null });
}

export async function getLeadDetail(leadId: number): Promise<Lead | null> {
  return invoke("get_lead_detail", { leadId });
}

export async function getLeadMessages(leadId: number): Promise<OutreachMessage[]> {
  return invoke("get_lead_messages", { leadId });
}

export async function setOutreachStatus(id: number, status: string) {
  return invoke("set_outreach_status", { id, status });
}

export async function enrichLeads(leadIds: number[], apiKey: string): Promise<{ enriched: number; failed: number }> {
  return invoke("enrich_leads_cmd", { leadIds, apiKey });
}

export async function enrichWebsites(leadIds: number[]): Promise<{ enriched: number; failed: number }> {
  return invoke("enrich_websites_cmd", { leadIds });
}

export async function updateLeadStatus(id: number, status: string) {
  return invoke("update_lead_status", { id, status });
}

export async function getDashboardStats(): Promise<DashboardStats> {
  return invoke<DashboardStats>("get_dashboard_stats");
}

export function friendlyError(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object") {
    const o = e as { message?: string };
    if (o.message) return o.message;
  }
  return "Unexpected error";
}
