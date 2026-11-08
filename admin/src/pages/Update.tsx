import { useEffect, useState } from "react";

import { adminRequest } from "../lib/api";

interface UpdateStatus {
  current: string;
  latest: string;
  has_update: boolean;
  notes: string;
  size: number;
  published_at: string;
  pending_version: string | null;
  can_install: boolean;
}

interface InstallResult {
  version: string;
  restart_required: boolean;
}

export default function UpdatePage() {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [installed, setInstalled] = useState<InstallResult | null>(null);

  async function reload() {
    setError(null);
    try {
      const data = await adminRequest<UpdateStatus>("/api/v1/admin/update");
      setStatus(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "查询版本失败");
    }
  }

  useEffect(() => {
    void reload();
  }, []);

  async function install() {
    setBusy(true);
    setNote("正在下载并校验安装包，完成后需要重启服务…");
    setError(null);

    try {
      const result = await adminRequest<InstallResult>("/api/v1/admin/update/install", {
        method: "POST"
      });
      setInstalled(result);
      setNote("新版本 " + result.version + " 已装好，重启后生效");
      await reload();
    } catch (err) {
      setNote(null);
      setError(err instanceof Error ? err.message : "安装失败");
    } finally {
      setBusy(false);
    }
  }

  async function restart() {
    setBusy(true);
    setNote("已通知服务重启，页面稍后会自动重连…");
    setError(null);

    try {
      await adminRequest("/api/v1/admin/update/restart", { method: "POST" });
    } catch {
      // 进程正在退出，请求断掉是正常的
    }

    // 等启动器把进程拉起来再重新查询
    for (let attempt = 0; attempt < 20; attempt++) {
      await new Promise((resolve) => setTimeout(resolve, 1500));
      try {
        await reload();
        setNote("服务已重启");
        setBusy(false);
        return;
      } catch {
        // 还没起来，继续等
      }
    }

    setNote("等了 30 秒还没连上，去服务器看下日志");
    setBusy(false);
  }

  return (
    <section>
      <header className="page-head">
        <h2>在线升级</h2>
        <button onClick={() => void reload()} disabled={busy}>
          刷新
        </button>
      </header>

      {error && <p className="error">{error}</p>}

      {status && (
        <>
          <div className="card">
            <div className="update-versions">
              <div>
                <span className="hint">当前版本</span>
                <strong>{status.current || "未知"}</strong>
              </div>
              <div>
                <span className="hint">最新版本</span>
                <strong>{status.latest || "未知"}</strong>
              </div>
            </div>

            {status.pending_version && (
              <p className="hint warn">
                已经装好 {status.pending_version}，重启服务后生效。
              </p>
            )}

            {!status.pending_version && !status.has_update && (
              <p className="hint">已经是最新版本。</p>
            )}

            {status.has_update && (
              <p className="hint">
                可以更新到 {status.latest}，安装包 {formatSize(status.size)}
                {status.published_at ? "，发布于 " + status.published_at.slice(0, 10) : ""}。
              </p>
            )}

            {status.notes && status.has_update && (
              <pre className="update-notes">{status.notes}</pre>
            )}

            <div className="update-actions">
              {status.has_update && (
                <button onClick={() => void install()} disabled={busy || !status.can_install}>
                  {busy ? "处理中…" : "下载并安装"}
                </button>
              )}

              {(status.pending_version || installed) && (
                <button className="danger" onClick={() => void restart()} disabled={busy}>
                  重启服务
                </button>
              )}
            </div>

            {!status.can_install && (
              <p className="hint">
                当前环境不支持自助安装（容器部署或程序目录不可写）。请拉新镜像或手动替换二进制。
              </p>
            )}

            {note && <p className="hint">{note}</p>}
          </div>

          <div className="card">
            <h3>手动更新</h3>
            <p className="hint">
              自助安装会用新包覆盖程序文件，旧版本留在同目录的 tududu-backup。偏好命令行的话：
            </p>
            <pre className="code">tududu update check
tududu update install --yes</pre>
            <p className="hint">
              装上启动器后（systemd 指向 tududu-launcher），进程以 75 退出会自动被拉起来，
              后台点「重启服务」即可。
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
