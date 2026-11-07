import { useEffect, useState } from "react";

import { BASE, request, requestBlob } from "../lib/api";

interface DownloadLink {
  url: string;
  expires_at: number;
}

type TauriGlobal = {
  core?: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> };
};

/**
 * 图片灯箱。
 *
 * 先用缩略图垫底（大概率已经在缓存里），再换成 1280 的预览图；
 * 原图只在下载时给。这样点开是「先看到模糊的，再变清楚」，不是干等。
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
  const [thumb, setThumb] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tip, setTip] = useState<string | null>(null);

  useEffect(() => {
    let revoked = false;
    const created: string[] = [];

    async function load() {
      try {
        // 缩略图先到，让画面立刻有东西
        const small = await requestBlob("/api/v1/assets/" + assetId + "/file?size=thumb");
        if (!revoked) {
          const url = URL.createObjectURL(small);
          created.push(url);
          setThumb(url);
        }

        const big = await requestBlob("/api/v1/assets/" + assetId + "/file?size=preview");
        if (!revoked) {
          const url = URL.createObjectURL(big);
          created.push(url);
          setPreview(url);
        }
      } catch {
        if (!revoked) setError("图片加载失败，重新打开试试");
      } finally {
        if (!revoked) setLoading(false);
      }
    }

    void load();

    return () => {
      revoked = true;
      for (const url of created) URL.revokeObjectURL(url);
    };
  }, [assetId]);

  async function download() {
    setTip(null);

    let target: string;
    try {
      const link = await request<DownloadLink>("/api/v1/assets/" + assetId + "/download-link");
      target = link.url.startsWith("http") ? link.url : BASE + link.url;
    } catch {
      setTip("生成下载链接失败，稍后再试");
      return;
    }

    const tauri = (window as Window & { __TAURI__?: TauriGlobal }).__TAURI__;

    if (tauri?.core?.invoke) {
      try {
        await tauri.core.invoke("open_external", { url: target });
        setTip("已在浏览器打开，长按图片即可保存");
      } catch {
        setTip("打开浏览器失败，复制链接到浏览器试试");
      }
      return;
    }

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
          {thumb && !preview && <img className="blur-up" src={thumb} alt="加载中" />}

          {preview && <img src={preview} alt="作品预览" />}

          {loading && !thumb && <div className="shimmer" />}
          {loading && thumb && <div className="lightbox-loading">正在加载清晰大图…</div>}
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

          <button className="lamp" onClick={() => void download()} aria-label="下载原图">
            <span aria-hidden="true">⤓</span>
            原图
          </button>
        </div>
      </div>
    </div>
  );
}
