const BASE: string = import.meta.env.VITE_API_BASE ?? "";

const TOKEN_KEY = "tududu.access";
const REFRESH_KEY = "tududu.refresh";

export interface ApiBody<T> {
  code: number;
  message: string;
  data: T | null;
}

export class ApiError extends Error {
  readonly code: number;

  constructor(code: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.code = code;
  }
}

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function saveTokens(access: string, refresh: string): void {
  localStorage.setItem(TOKEN_KEY, access);
  localStorage.setItem(REFRESH_KEY, refresh);
}

export function clearTokens(): void {
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(REFRESH_KEY);
}

export function getRefreshToken(): string | null {
  return localStorage.getItem(REFRESH_KEY);
}

interface RequestOptions {
  method?: string;
  body?: unknown;
  form?: FormData;
  auth?: boolean;
}

export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = {};
  const token = getToken();
  if (options.auth !== false && token) {
    headers.Authorization = `Bearer ${token}`;
  }

  let payload: BodyInit | undefined;
  if (options.form) {
    payload = options.form;
  } else if (options.body !== undefined) {
    headers["Content-Type"] = "application/json";
    payload = JSON.stringify(options.body);
  }

  const response = await fetch(`${BASE}${path}`, {
    method: options.method ?? "GET",
    headers,
    body: payload
  });

  const text = await response.text();
  let parsed: ApiBody<T> | null = null;
  if (text) {
    try {
      parsed = JSON.parse(text) as ApiBody<T>;
    } catch {
      parsed = null;
    }
  }

  if (!response.ok || !parsed || parsed.code !== 0 || parsed.data === null) {
    const code = parsed?.code ?? response.status;
    const message = parsed?.message ?? `请求失败（${response.status}）`;
    throw new ApiError(code, message);
  }

  return parsed.data;
}
