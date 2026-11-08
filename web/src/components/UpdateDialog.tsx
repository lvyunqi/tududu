import { useEffect, useState } from "react";

import {
  downloadUpdate,
  formatSize,
  installUpdate,
  openInBrowser,
  type Progress,
  type VersionInfo
} from "../lib/updater";

type Stage = "idle" | "downloading" | "done" | "error";

/**
 * 发现新版本时的弹窗。
 *
 * 点「立即更新」走应用内下载，进度条实时走；下完自动拉起系统安装界面。
 * 应用内下载失败时退回系统浏览器，用户手动装。
 */
export default function UpdateDialog({ info, onClose }: { info: VersionInfo; onClose: () => void }) {
  const [stage, setStage] = useState<Stage>("idle");
  const [progress, setProgress] = useState<Progress | null>(null);
  const [note, setNote] = useState<string | null>(null);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape" && stage !== "downloading") onClose();
    }

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, stage]);

  async function start() {
    setStage("downloading");
    setNote(null);

    try {
      const path = await downloadUpdate(info, setProgress);
      setStage("done");

      try {
        await installUpdate(path);
      } catch {
        setNote("已经下载好，但没拉起安装界面，正在用浏览器再试一次");
        await openInBrowser(info);
      }
    } catch (err) {
      setStage("error");
      setNote(err instanceof Error ? err.message : "下载失败");
    }
  }

  const percent =
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100))
      : null;

  return (
    <div className="sheet-backdrop" onClick={() => stage !== "downloading" && onClose()}>
      <div className="sheet update-sheet" onClick={(event) => event.stopPropagation()}>
        <div className="update-head">
          <span className="update-badge">新版本</span>
          <strong>{info.latest}</strong>
          <span className="hint">{formatSize(info.size)}</span>
        </div>

        {info.notes && <p className="update-notes">{info.notes}</p>}

        {stage === "downloading" && (
          <div className="update-progress">
            <div className="meter">
              <span style={{ width: (percent ?? 4) + "%" }} />
            </div>
            <p className="hint">
              {percent === null
                ? "正在下载…"
                : "正在下载 " + percent + "% · " + formatSize(progress?.downloaded ?? 0)}
            </p>
          </div>
        )}

        {stage === "done" && <p className="hint">下载完成，正在打开安装界面…</p>}
        {note && <p className="hint">{note}</p>}

        <div className="update-actions">
          {stage === "idle" && (
            <>
              <button className="ghost" onClick={onClose}>
                稍后
              </button>
              <button onClick={() => void start()}>立即更新</button>
            </>
          )}

          {stage === "downloading" && <button className="ghost" disabled>请稍候…</button>}

          {(stage === "done" || stage === "error") && (
            <>
              <button className="ghost" onClick={onClose}>
                关闭
              </button>
              <button onClick={() => void openInBrowser(info)}>用浏览器下载</button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
