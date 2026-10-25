import { useEffect, useState } from "react";

import { requestBlob } from "../lib/api";

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
    if (!url) return;
    setTip(null);

    const filename = "tududu-" + assetId.slice(0, 8) + ".png";

    // 部分 WebView 会忽略 download 属性，先试系统分享，能直接存相册
    const nav = navigator as Navigator & {
      share?: (data: ShareData) => Promise<void>;
      canShare?: (data: ShareData) => boolean;
    };

    try {
      const blob = await fetch(url).then((response) => response.blob());
      const file = new File([blob], filename, { type: blob.type || "image/png" });

      if (nav.share && nav.canShare?.({ files: [file] })) {
        await nav.share({ files: [file] });
        return;
      }
    } catch {
      // 用户取消分享，不算失败
    }

    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    link.rel = "noopener";
    document.body.appendChild(link);
    link.click();
    link.remove();

    setTip("没开始下载的话，长按图片可以保存到相册");
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
            disabled={!url}
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
