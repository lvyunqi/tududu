/// 打包成 App 时前端不在服务端同源，需要通过 VITE_API_BASE 指定后端地址。
export const BASE: string = import.meta.env.VITE_API_BASE ?? "";

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

// 同一时间只允许一次刷新，避免多个请求同时进来把刷新令牌用掉又互相覆盖
let rotating: Promise<boolean> | null = null;

/**
 * 用刷新令牌换一套新的。
 *
 * 访问令牌只有 15 分钟，而刷新令牌有 30 天。没有这一步的话，
 * 用户关掉页面超过 15 分钟再打开就会被判成未登录。
 */
async function rotateTokens(): Promise<boolean> {
  if (rotating) return rotating;

  const task = (async () => {
    const refresh = getRefreshToken();
    if (!refresh) return false;

    try {
      const response = await fetch(BASE + "/api/v1/auth/refresh", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ refresh_token: refresh })
      });

      if (!response.ok) return false;

      const parsed = (await response.json()) as ApiBody<{
        access_token: string;
        refresh_token: string;
      }>;

      if (parsed.code !== 0 || !parsed.data) return false;

      saveTokens(parsed.data.access_token, parsed.data.refresh_token);
      return true;
    } catch {
      return false;
    }
  })();

  rotating = task;

  try {
    return await task;
  } finally {
    if (rotating === task) rotating = null;
  }
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

  const url = BASE + path;
  const init: RequestInit = {
    method: options.method ?? "GET",
    headers,
    body: payload
  };

  let response = await fetch(url, init);

  // 访问令牌过期：换一套再重试一次，用户不用重新登录
  if (response.status === 401 && options.auth !== false && !path.startsWith("/api/v1/auth/refresh")) {
    if (await rotateTokens()) {
      const retryHeaders: Record<string, string> = { ...headers };
      const token = getToken();
      if (token) retryHeaders.Authorization = `Bearer ${token}`;
      response = await fetch(url, { ...init, headers: retryHeaders });
    } else {
      clearTokens();
    }
  }

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
