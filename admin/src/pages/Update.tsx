import { useEffect, useRef, useState } from "react";

import { adminRequest } from "../lib/api";

interface LatestRelease {
  version: string;
  tag: string;
  package_name: string;
  notes: string;
  size: number;
  published_at: string;
  sha256: string | null;
}

interface SoftwareStatus {
  version: string;
  commit: string;
  deployment: "binary" | "container";
  launch_mode: "launcher" | "bare";
  install_enabled: boolean;
  check_enabled: boolean;
  update_phase: string;
  error: string | null;
  update_available: boolean;
  latest: LatestRelease | null;
  pending_version: string | null;
  checked_at: number | null;
  blocked: string | null;
}

const BLOCKED_TEXT: Record<string, string> = {
  container: "容器部署不能自助升级，换镜像后重新部署即可。",
  not_enabled: "自助升级被 TUDUDU_DISABLE_SELF_UPDATE 关掉了。",
  host_managed: "程序目录不可写，请手动替换二进制。",
  executable_unavailable: "读不到当前程序路径，无法替换。"
};

const PHASE_TEXT: Record<string, string> = {
  preparing: "准备中…",
  downloading: "正在下载安装包…",
  verifying: "正在校验 SHA-256…",
  installing: "正在替换程序…",
  restart_required: "新版本已装好，重启服务后生效",
  restarting: "正在重启…",
  failed: "安装失败"
};

const BUSY_PHASES = ["preparing", "downloading", "verifying", "installing", "restarting"];

export default function UpdatePage() {
  const [status, setStatus] = useState<SoftwareStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmed, setConfirmed] = useState(false);
  const [notesOpen, setNotesOpen] = useState(false);
  const timer = useRef<number | null>(null);

  async function load(refresh = false) {
    try {
      const data = await adminRequest<SoftwareStatus>(
        "/api/v1/admin/update" + (refresh ? "?refresh=1" : "")
      );
      setStatus(data);
      setError(null);
      return data;
    } catch (err) {
      setError(err instanceof Error ? err.message : "查询版本失败");
      return null;
    }
  }

  useEffect(() => {
    void load();
  }, []);

  // 装的过程中每 2 秒轮询一次，直到进入待重启或失败
  useEffect(() => {
    if (!status) return;
    const busyNow = BUSY_PHASES.includes(status.update_phase);
    if (!busyNow) return;

    timer.current = window.setInterval(() => void load(), 2000);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
  }, [status?.update_phase]);

  async function check() {
    setBusy(true);
    setError(null);
    setNote(null);
    setConfirmed(false);

    try {
      const data = await adminRequest<SoftwareStatus>("/api/v1/admin/update/check", {
        method: "POST"
      });
      setStatus(data);
      if (data.update_available && data.latest?.notes.trim()) setNotesOpen(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "检查更新失败");
    } finally {
      setBusy(false);
    }
  }

  async function install() {
    setBusy(true);
    setError(null);
    setNote(null);

    try {
      await adminRequest("/api/v1/admin/update/install", {
        method: "POST",
        body: { tag: status?.latest?.tag }
      });
      setNote("已开始安装，进度会自动刷新");
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "安装失败");
    } finally {
      setBusy(false);
    }
  }

  async function restart() {
    setBusy(true);
    setNote("已通知服务重启，正在等它起来…");

    try {
      await adminRequest("/api/v1/admin/update/restart", { method: "POST" });
    } catch {
      // 进程正在退出，请求断掉是正常的
    }

    for (let attempt = 0; attempt < 24; attempt++) {
      await new Promise((resolve) => setTimeout(resolve, 1500));
      const data = await load();
      if (data) {
        setNote("服务已重启，当前版本 " + data.version);
        setBusy(false);
        return;
      }
    }

    setNote("等了 40 秒还没连上，去服务器看下日志");
    setBusy(false);
  }

  const phase = status?.update_phase ?? "idle";
  const phaseText = PHASE_TEXT[phase] ?? "";
  const waitingRestart = phase === "restart_required" || Boolean(status?.pending_version);
  const blockedText = status?.blocked ? BLOCKED_TEXT[status.blocked] ?? "当前环境不能自助升级。" : null;
  const canInstall = Boolean(
    status?.install_enabled && status.update_available && status.latest?.package_name
  );

  return (
    <section>
      <header className="page-head">
        <h2>系统版本与更新</h2>
        <button onClick={() => void check()} disabled={busy || Boolean(phaseText && BUSY_PHASES.includes(phase))}>
          {busy ? "检查中…" : "检查更新"}
        </button>
      </header>

      {error && <p className="error">{error}</p>}

      {status && (
        <>
          <div className="card">
            <div className="update-versions">
              <div>
                <span className="hint">当前版本</span>
                <strong>{status.version}</strong>
                <span className="hint">
                  {status.deployment === "binary" ? "二进制部署" : "容器部署"} ·{" "}
                  {status.launch_mode === "launcher" ? "启动器托管" : "直接运行"}
                </span>
              </div>
              <div>
                <span className="hint">最新版本</span>
                <strong>{status.latest?.version ?? "未知"}</strong>
                <span className="hint">
                  {status.checked_at
                    ? "检查于 " + new Date(status.checked_at * 1000).toLocaleString()
                    : "还没检查过"}
                </span>
              </div>
            </div>

            {status.commit && (
              <p className="hint">构建提交：<code>{status.commit.slice(0, 12)}</code></p>
            )}

            {phaseText && <p className={phase === "failed" ? "error" : "hint warn"}>{phaseText}</p>}
            {status.error && <p className="error">{status.error}</p>}

            {!waitingRestart && !status.update_available && (
              <p className="hint">已经是最新版本。</p>
            )}

            {status.update_available && status.latest && (
              <>
                <p className="hint">
                  可以更新到 {status.latest.version}，包名{" "}
                  <code>{status.latest.package_name}</code>，大小 {formatSize(status.latest.size)}
                  {status.latest.published_at
                    ? "，发布于 " + status.latest.published_at.slice(0, 10)
                    : ""}。
                </p>

                {status.latest.notes.trim() && (
                  <>
                    <button className="link" onClick={() => setNotesOpen((open) => !open)}>
                      {notesOpen ? "收起更新说明" : "查看更新说明"}
                    </button>
                    {notesOpen && <pre className="update-notes">{status.latest.notes}</pre>}
                  </>
                )}

                <div className="update-actions">
                  <label className="checkbox">
                    <input
                      type="checkbox"
                      checked={confirmed}
                      onChange={(event) => setConfirmed(event.target.checked)}
                      disabled={!canInstall}
                    />
                    我知道升级会短暂中断服务，旧版本会备份成 tududu-backup
                  </label>
                  <button onClick={() => void install()} disabled={busy || !canInstall || !confirmed}>
                    下载并安装
                  </button>
                </div>
              </>
            )}

            {waitingRestart && (
              <>
                <p className="hint warn">
                  {status.pending_version ?? status.latest?.version} 已装好，重启服务后生效。
                  {status.launch_mode === "launcher"
                    ? "服务由启动器托管，重启会自动拉起来。"
                    : "当前是直接运行，重启后需要你手动或由服务管理器拉起。"}
                </p>
                <div className="update-actions">
                  <button className="danger" onClick={() => void restart()} disabled={busy}>
                    重启服务
                  </button>
                </div>
              </>
            )}

            {blockedText && <p className="hint">{blockedText}</p>}
            {note && <p className="hint">{note}</p>}
          </div>

          <div className="card">
            <h3>手动更新</h3>
            <p className="hint">
              自助安装会覆盖程序文件，旧版本留在同目录的 tududu-backup，出问题可以手工换回去。
            </p>
            <pre className="code">tududu update check
tududu update install --yes</pre>
            <p className="hint">
              自助升级可以直接关掉：环境变量设 TUDUDU_DISABLE_SELF_UPDATE=1。
            </p>
          </div>
        </>
      )}

      {!status && !error && <p className="hint">正在查询版本…</p>}
    </section>
  );
}

function formatSize(bytes: number): string {
  if (!bytes) return "未知大小";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(0) + " KB";
  return (bytes / 1024 / 1024).toFixed(1) + " MB";
}
