import { useEffect, useState } from "react";

import { renderMarkdown } from "../lib/markdown";
import { formatSize, openInBrowser, type VersionInfo } from "../lib/updater";

/**
 * 发现新版本时的弹窗。
 *
 * 只负责讲清楚更新了什么；点「去下载」交给系统浏览器，
 * 下载进度和安装都由系统接管，不在 App 里重复造一遍。
 */
export default function UpdateDialog({ info, onClose }: { info: VersionInfo; onClose: () => void }) {
  const [note, setNote] = useState<string | null>(null);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  async function goDownload() {
    setNote(null);

    try {
      await openInBrowser(info);
      onClose();
    } catch (err) {
      setNote(err instanceof Error ? err.message : "打开浏览器失败");
    }
  }

  return (
    <div className="sheet-backdrop" onClick={onClose}>
      <div className="sheet update-sheet" onClick={(event) => event.stopPropagation()}>
        <div className="update-head">
          <span className="update-badge">新版本</span>
          <strong>{info.latest}</strong>
          <span className="hint">{formatSize(info.size)}</span>
        </div>

        {info.notes && <div className="update-notes md">{renderMarkdown(info.notes)}</div>}

        {note && <p className="hint">{note}</p>}

        <div className="update-actions">
          <button className="ghost" onClick={onClose}>
            稍后
          </button>
          <button onClick={() => void goDownload()}>去下载</button>
        </div>
      </div>
    </div>
  );
}
