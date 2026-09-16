export type Lead = {
  id: number;
  canonical_name: string;
  category: string | null;
  address: string | null;
  latitude: number | null;
  longitude: number | null;
  phone: string | null;
  website: string | null;
  rating: number | null;
  review_count: number | null;
  email: string | null;
  instagram: string | null;
  facebook: string | null;
  whatsapp: string | null;
  website_status: string | null;
  score: number;
  score_reasons: string | null;
  lead_status: string;
  follow_up_at: string | null;
  created_at: string;
  updated_at: string;
};

export const PIPELINE = ["new","qualified","message_ready","contacted","replied","interested","meeting","won","lost","skipped","do_not_contact"] as const;

export type StatusEvent = {
  id: number;
  from_status: string | null;
  to_status: string;
  note: string | null;
  created_at: string;
};

export type LeadNote = {
  id: number;
  note: string;
  follow_up_at: string | null;
  created_at: string;
};

export type ProviderCount = { provider: string; leads: number; discoveries: number };

export type ScoreConfig = {
  no_website: number;
  website_offline: number;
  has_instagram: number;
  has_phone: number;
  has_email: number;
  rating_gte: number;
  reviews_gte: number;
  active_bonus: number;
  rating_threshold: number;
  reviews_threshold: number;
};

export type SavedFilter = {
  id: number;
  name: string;
  filters_json: string;
  created_at: string;
};

export type MessageTemplate = {
  id: string;
  label: string;
  channel: string;
  subject: string;
  body: string;
};

export type OutreachMessage = {
  id: number;
  lead_id: number;
  channel: string;
  subject: string | null;
  message: string;
  template_id: string | null;
  provider: string;
  status: string;
  created_at: string;
};

export type DashboardStats = {
  total_leads: number;
  new_leads: number;
  searches_completed: number;
  qualified: number;
  contacted: number;
  replied: number;
  won: number;
  without_website: number;
  with_email: number;
};

export type SearchResult = {
  job_id: number;
  result_count: number;
  new_count: number;
};

export type SearchJob = {
  id: number;
  query: string;
  city: string;
  radius_meters: number | null;
  center_lat: number | null;
  center_lng: number | null;
  strategy: string;
  status: string;
  result_count: number;
  new_count: number;
  total_cells: number;
  completed_cells: number;
  coverage: number;
  created_at: string;
};

export type SearchCell = {
  id: number;
  job_id: number;
  parent_id: number | null;
  north: number;
  south: number;
  east: number;
  west: number;
  depth: number;
  status: string;
  raw_result_count: number;
  new_unique_count: number;
  query: string;
  provider: string;
};
