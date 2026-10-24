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

export async function adminRequest<T>(path: string, options: Options = {}): Promise<T> {
  const token = getAdminToken();
  const headers: Record<string, string> = token.startsWith("ey") 
    ? { Authorization: `Bearer ${token}` }
    : { "x-admin-token": token };

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
