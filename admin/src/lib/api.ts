const TOKEN_KEY = "tududu.admin.token";

export interface ApiBody<T> {
  code: number;
  message: string;
  data: T | null;
}

export class AdminApiError extends Error {
  readonly code: number;

  constructor(code: number, message: string) {
    super(message);
    this.name = "AdminApiError";
    this.code = code;
  }
}

export function getAdminToken(): string {
  return localStorage.getItem(TOKEN_KEY) ?? "";
}

export function setAdminToken(token: string): void {
  if (token) {
    localStorage.setItem(TOKEN_KEY, token);
  } else {
    localStorage.removeItem(TOKEN_KEY);
  }
}

interface Options {
  method?: string;
  body?: unknown;
}

/// 进行中的请求数，App 用它显示顶部进度条。
let pending = 0;
const listeners = new Set<(count: number) => void>();

function notify() {
  for (const listener of listeners) listener(pending);
}

export function subscribePending(listener: (count: number) => void): () => void {
  listeners.add(listener);
  listener(pending);
  return () => {
    listeners.delete(listener);
  };
}

export async function adminRequest<T>(path: string, options: Options = {}): Promise<T> {
  pending += 1;
  notify();

  try {
    return await send<T>(path, options);
  } finally {
    pending -= 1;
    notify();
  }
}

function authHeaders(extra: Record<string, string> = {}): Record<string, string> {
  const token = getAdminToken();
  const head: Record<string, string> = token.startsWith("ey")
    ? { Authorization: `Bearer ${token}` }
    : { "x-admin-token": token };

  return { ...head, ...extra };
}

/// 取图片字节，带上管理员令牌，不然被 401 挡住。
export async function adminBlob(path: string): Promise<Blob> {
  const response = await fetch(path, { headers: authHeaders() });

  if (!response.ok) {
    throw new AdminApiError(response.status, `图片读取失败（${response.status}）`);
  }

  return response.blob();
}

async function send<T>(path: string, options: Options = {}): Promise<T> {
  const headers = authHeaders();

  let payload: BodyInit | undefined;
  if (options.body !== undefined) {
    headers["Content-Type"] = "application/json";
    payload = JSON.stringify(options.body);
  }

  const response = await fetch(path, {
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

  if (!response.ok || !parsed || parsed.code !== 0) {
    throw new AdminApiError(
      parsed?.code ?? response.status,
      parsed?.message ?? `请求失败（${response.status}）`
    );
  }

  return parsed.data as T;
}
