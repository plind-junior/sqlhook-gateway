import axios from 'axios';

const apiClient = axios.create({
  baseURL: '/api',
  headers: { 'Content-Type': 'application/json' },
});

// ----- types -----

export type JobStatus = 'pending' | 'processing' | 'done' | 'dead';

export interface Job {
  id: string;
  route_id: string;
  source_path: string;
  status: JobStatus;
  attempts: number;
  last_error: string | null;
  last_response_code: number | null;
  visible_at: string;
  created_at: string;
  updated_at: string;
}

export interface Delivery {
  id: string;
  job_id: string;
  route_id: string;
  attempt: number;
  transformed_payload: string | null;
  destination_url: string;
  success: number; // 0 or 1
  response_code: number | null;
  response_body: string | null;
  error: string | null;
  duration_ms: number;
  created_at: string;
}

export interface RouteView {
  id: string;
  source_path: string;
  destination_url: string;
  timeout_ms: number;
  max_attempts: number;
  initial_backoff_ms: number;
  max_backoff_ms: number;
  signature_header: string;
}

export interface Stats {
  jobs_pending: number;
  jobs_processing: number;
  jobs_done: number;
  jobs_dead: number;
  deliveries_total: number;
  deliveries_success: number;
}

// ----- API -----

export const statsApi = {
  get: async (): Promise<Stats> => {
    const { data } = await apiClient.get<{ stats: Stats }>('/stats');
    return data.stats;
  },
};

export interface JobListParams {
  status?: JobStatus;
  route?: string;
  limit?: number;
  offset?: number;
}

export const jobsApi = {
  list: async (params: JobListParams = {}): Promise<Job[]> => {
    const { data } = await apiClient.get<{ jobs: Job[] }>('/jobs', { params });
    return data.jobs;
  },

  get: async (id: string): Promise<Job> => {
    const { data } = await apiClient.get<{ job: Job }>(`/jobs/${id}`);
    return data.job;
  },

  deliveries: async (id: string): Promise<Delivery[]> => {
    const { data } = await apiClient.get<{ deliveries: Delivery[] }>(`/jobs/${id}/deliveries`);
    return data.deliveries;
  },

  replay: async (id: string): Promise<void> => {
    await apiClient.post(`/jobs/${id}/replay`);
  },
};

export const routesApi = {
  list: async (): Promise<RouteView[]> => {
    const { data } = await apiClient.get<{ routes: RouteView[] }>('/routes');
    return data.routes;
  },
};

export default apiClient;
