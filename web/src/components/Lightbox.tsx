import { useEffect, useState } from "react";

import { BASE, request, requestBlob } from "../lib/api";

interface DownloadLink {
  url: string;
  expires_at: number;
}

/** Tauri 打开 withGlobalTauri 后，WebView 里能直接拿到注入的接口。 */
type TauriGlobal = {
  core?: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> };
};

/**
 * 图片灯箱：取原图、可下载、可以继续改或发布。
 *
 * 列表里用的是缩略图，这里取原图，免得点开看到的还是糊的那张。
 */
export default function Lightbox({
  assetId,
  onClose,
  onEdit,
  onPublish
}: {
  assetId: string;
  onClose: () => void;
  onEdit?: () => void;
  onPublish?: () => void;
}) {
  const [url, setUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tip, setTip] = useState<string | null>(null);

  useEffect(() => {
    let revoked = false;
    let objectUrl: string | null = null;

    async function load() {
      try {
        const blob = await requestBlob("/api/v1/assets/" + assetId + "/file");
        objectUrl = URL.createObjectURL(blob);
        if (!revoked) setUrl(objectUrl);
      } catch {
        if (!revoked) setError("图片加载失败，重新打开试试");
      } finally {
        if (!revoked) setLoading(false);
      }
    }

    void load();

    return () => {
      revoked = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [assetId]);

  async function download() {
    setTip(null);

    // 后端给一个十分钟内有效的直链，浏览器不需要带请求头
    let target: string;
    try {
      const link = await request<DownloadLink>("/api/v1/assets/" + assetId + "/download-link");
      target = link.url.startsWith("http") ? link.url : BASE + link.url;
    } catch {
      setTip("生成下载链接失败，稍后再试");
      return;
    }

    const tauri = (window as Window & { __TAURI__?: TauriGlobal }).__TAURI__;

    // App 内：交给系统浏览器，浏览器能正常保存图片
    if (tauri?.core?.invoke) {
      try {
        await tauri.core.invoke("open_external", { url: target });
        setTip("已在浏览器打开，长按图片即可保存");
      } catch {
        setTip("打开浏览器失败，复制链接到浏览器试试");
      }
      return;
    }

    // 网页版：直接下载
    const link = document.createElement("a");
    link.href = target;
    link.download = "tududu-" + assetId.slice(0, 8) + ".png";
    link.rel = "noopener";
    document.body.appendChild(link);
    link.click();
    link.remove();
  }

  return (
    <div className="lightbox" onClick={onClose}>
      <div className="lightbox-body" onClick={(event) => event.stopPropagation()}>
        <div className="lightbox-stage">
          {loading && <div className="shimmer" />}
          {!loading && url && <img src={url} alt="作品原图" />}
          {error && <p className="hint">{error}</p>}
        </div>

        {tip && <p className="lightbox-tip">{tip}</p>}

        <div className="lightbox-actions">
          <button className="lamp" onClick={onClose} aria-label="关闭">
            <span aria-hidden="true">✕</span>
            关闭
          </button>

          {onEdit && (
            <button className="lamp" onClick={onEdit} aria-label="以这张继续改">
              <span aria-hidden="true">✎</span>
              接着改
            </button>
          )}

          {onPublish && (
            <button className="lamp strong" onClick={onPublish} aria-label="发布到广场">
              <span aria-hidden="true">↗</span>
              发布
            </button>
          )}

          <button
            className="lamp"
            onClick={() => void download()}
            aria-label="下载"
          >
            <span aria-hidden="true">⤓</span>
            下载
          </button>
        </div>
      </div>
    </div>
  );
}
