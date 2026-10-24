export interface UserInfo {
  id: string;
  short_code: string;
  nickname: string;
  phone: string | null;
  email: string | null;
  created_at: string;
}

export interface AuthResponse {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  user: UserInfo;
}

export interface Wallet {
  user_id: string;
  balance: number;
  total_charged: number;
}

export interface BillingConfig {
  unit_name: string;
  unit_per_yuan: number;
}

export interface Asset {
  id: string;
  mime: string;
  byte_size: number;
  source: string;
  status: number;
  created_at: string;
}

export interface Session {
  id: string;
  title: string;
  model: string;
  status: number;
  last_active_at: string;
  created_at: string;
}

export interface Message {
  id: string;
  role: string;
  content: string;
  input_asset_id: string | null;
  output_asset_id: string | null;
  job_id: string | null;
  status: number;
  created_at: string;
}

export interface SessionDetail {
  session: Session;
  messages: Message[];
}

export interface TurnResult {
  user_message_id: string;
  assistant_message_id: string;
  job_id: string;
}

export interface Job {
  id: string;
  status: number;
  output_asset_id: string | null;
  error_message: string | null;
  price: number;
}

export interface PromptItem {
  id: string;
  short_code: string;
  author_user_id: string;
  title: string;
  prompt_text: string;
  model: string;
  cover_asset_id: string | null;
  like_count: number;
  view_count: number;
  published_at: string;
}

export interface PromptCategory {
  id: number;
  name: string;
  slug: string;
}
