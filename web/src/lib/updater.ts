import { BASE } from "./api";

/** 服务端 /api/v1/app/version 的返回。 */
export interface VersionInfo {
  latest: string;
  current: string;
  has_update: boolean;
  notes: string;
  published_at: string;
  size: number;
  download_url: string;
}

interface TauriGlobal {
  app?: { getVersion: () => Promise<string> };
  core?: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> };
  event?: {
    listen: (name: string, handler: (event: { payload: Progress }) => void) => Promise<() => void>;
  };
}

export interface Progress {
  downloaded: number;
  total: number;
}

/** 在 App 里才有原生能力；浏览器里返回 null。 */
export function tauri(): TauriGlobal | null {
  const global = (window as Window & { __TAURI__?: TauriGlobal }).__TAURI__;
  return global?.core ? global : null;
}

/** 当前应用版本，浏览器里退回一个空串表示不检查。 */
export async function currentVersion(): Promise<string> {
  const api = tauri();
  if (!api?.app?.getVersion) return "";

  try {
    return await api.app.getVersion();
  } catch {
    return "";
  }
}

/** 问服务端有没有新版本。出错就当没有，不能因为查版本卡住启动。 */
export async function checkUpdate(platform: string): Promise<VersionInfo | null> {
  const current = await currentVersion();

  try {
    const params = new URLSearchParams({ platform });
    if (current) params.set("current", current);

    const response = await fetch(BASE + "/api/v1/app/version?" + params.toString());
    if (!response.ok) return null;

    const body = (await response.json()) as { data?: VersionInfo };
    const info = body.data;
    if (!info || !info.has_update) return null;

    return info;
  } catch {
    return null;
  }
}

/**
 * 应用内下载安装包，带进度回调。
 *
 * 浏览器里没有原生写入能力，直接在这里报错，让界面提供「用浏览器下载」的兜底。
 */
export async function downloadUpdate(
  info: VersionInfo,
  onProgress: (progress: Progress) => void
): Promise<string> {
  const api = tauri();
  if (!api?.core) {
    throw new Error("当前环境不支持应用内下载");
  }

  const url = info.download_url.startsWith("http") ? info.download_url : BASE + info.download_url;
  const name = "tududu-update-" + (info.latest || "latest") + ".apk";

  const off = await api.event?.listen("update://progress", (event) => onProgress(event.payload));

  try {
    const path = await api.core.invoke("download_update", { url, fileName: name });
    return String(path);
  } finally {
    off?.();
  }
}

/** 拉起系统安装界面；失败时把错误抛给调用方兜底。 */
export async function installUpdate(path: string): Promise<void> {
  const api = tauri();
  if (!api?.core) throw new Error("当前环境不支持安装");

  await api.core.invoke("install_update", { path });
}

/** 用系统浏览器下载，兜底方案。 */
export async function openInBrowser(info: VersionInfo): Promise<void> {
  const api = tauri();
  const url = info.download_url.startsWith("http") ? info.download_url : BASE + info.download_url;

  if (api?.core) {
    await api.core.invoke("open_external", { url });
    return;
  }

  window.location.href = url;
}

export function formatSize(bytes: number): string {
  if (!bytes) return "未知大小";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(0) + " KB";
  return (bytes / 1024 / 1024).toFixed(1) + " MB";
}
