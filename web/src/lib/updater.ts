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
  sha256?: string | null;
  build_commit?: string | null;
}

interface TauriGlobal {
  app?: { getVersion: () => Promise<string> };
  core?: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> };
}

interface BuildInfo {
  version: string;
  commit: string;
}

/** 在 App 里才有原生能力；浏览器里返回 null。 */
export function tauri(): TauriGlobal | null {
  const global = (window as Window & { __TAURI__?: TauriGlobal }).__TAURI__;
  return global?.core ? global : null;
}

/** 当前版本与构建提交；浏览器里返回空，表示不做检查。 */
export async function currentBuild(): Promise<BuildInfo> {
  const api = tauri();
  if (!api?.core) return { version: "", commit: "" };

  try {
    const info = (await api.core.invoke("app_build_info")) as BuildInfo;
    return { version: info?.version ?? "", commit: info?.commit ?? "" };
  } catch {
    const version = api.app?.getVersion ? await api.app.getVersion().catch(() => "") : "";
    return { version, commit: "" };
  }
}

/** 问服务端有没有新版本。出错就当没有，不能因为查版本卡住启动。 */
export async function checkUpdate(platform: string): Promise<VersionInfo | null> {
  const build = await currentBuild();

  try {
    const params = new URLSearchParams({ platform });
    if (build.version) params.set("current", build.version);
    if (build.commit) params.set("commit", build.commit);

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

/** 服务端给的是相对地址，拼成绝对地址。 */
export function absoluteUrl(path: string): string {
  if (path.startsWith("http")) return path;

  const base = BASE || window.location.origin;
  return base.replace(/\/$/, "") + path;
}

/**
 * 交给系统浏览器下载。
 *
 * WebView 里自己下载再拉起安装要过 FileProvider 那一套，安卓各版本行为还不一致；
 * 交给浏览器，进度和安装都由系统接管，最稳。
 */
export async function openInBrowser(info: VersionInfo): Promise<void> {
  const url = absoluteUrl(info.download_url);
  const api = tauri();

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
