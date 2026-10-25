export interface Overview {
  users_total: number;
  users_today: number;
  paid_orders_today: number;
  revenue_fen_today: number;
  jobs_today: number;
  jobs_failed_today: number;
  prompts_published: number;
}

export interface AdminUser {
  id: string;
  short_code: string;
  nickname: string;
  phone: string | null;
  email: string | null;
  status: number;
  balance: number;
  used_bytes: number;
  created_at: string;
}

export interface AdminOrder {
  order_no: string;
  user_id: string;
  amount_fen: number;
  credit: number;
  bonus: number;
  status: number;
  paid_at: string | null;
  created_at: string;
}

export interface AdminJob {
  id: string;
  user_id: string;
  model: string;
  status: number;
  price: number;
  error_code: number | null;
  error_message: string | null;
  finished_at: string | null;
  created_at: string;
}

export interface Attempt {
  attempt_no: number;
  channel_id: string | null;
  status: number;
  http_status: number | null;
  latency_ms: number | null;
  error_code: string | null;
  created_at: string;
}

export interface JobDetail {
  job: AdminJob;
  attempts: Attempt[];
}

export interface AdminPrompt {
  id: string;
  short_code: string;
  author_user_id: string;
  title: string;
  prompt_text: string;
  model: string;
  like_count: number;
  view_count: number;
  published_at: string;
}

export interface Provider {
  id: string;
  code: string;
  name: string;
  protocol: string;
  base_url: string;
  timeout_ms: number;
  max_concurrency: number;
  enabled: boolean;
  created_at: string;
}

export interface Channel {
  id: string;
  provider_id: string;
  name: string;
  api_key_hint: string;
  weight: number;
  priority: number;
  max_concurrency: number;
  health: number;
  enabled: boolean;
}

export interface Model {
  id: string;
  code: string;
  display_name: string;
  enabled: boolean;
}

export interface PriceRow {
  id: string;
  model: string;
  size: string | null;
  quality: string | null;
  price: number;
  enabled: boolean;
}

export interface BillingConfig {
  unit_name: string;
  unit_per_yuan: number;
}

export const JOB_STATUS: Record<number, string> = {
  0: "排队",
  1: "执行中",
  2: "成功",
  3: "失败",
  4: "取消"
};

export const ORDER_STATUS: Record<number, string> = {
  0: "待支付",
  1: "已支付",
  2: "已关闭"
};

export const USER_STATUS: Record<number, string> = {
  1: "正常",
  2: "禁言",
  3: "封禁"
};

export interface SensitiveWord {
  id: number;
  word: string;
  enabled: boolean;
}

export interface SettingItem {
  key: string;
  value: string;
  description: string;
  from_db: boolean;
}

export interface PayChannel {
  id: string;
  name: string;
  gateway_url: string;
  pid: string;
  pay_type: string;
  enabled: boolean;
  created_at: string;
}
