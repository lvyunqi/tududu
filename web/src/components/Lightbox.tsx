import { useEffect, useState } from "react";

import { BASE, getToken } from "../lib/api";

/**
 * 图片灯箱：加载原图、可下载、可以继续改或发布。
 *
 * 列表里用的是缩略图，这里带 scale=full 取原图，
 * 免得点开看到的还是糊的那张。
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

  useEffect(() => {
    let revoked = false;
    let objectUrl: string | null = null;

    async function load() {
      try {
        const token = getToken();
        const response = await fetch(BASE + "/api/v1/assets/" + assetId + "/file", {
          headers: token ? { Authorization: "Bearer " + token } : {}
        });

        if (!response.ok) {
          setError("图片加载失败");
          return;
        }

        const blob = await response.blob();
        objectUrl = URL.createObjectURL(blob);
        if (!revoked) setUrl(objectUrl);
      } catch {
        setError("图片加载失败");
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

  function download() {
    if (!url) return;

    const link = document.createElement("a");
    link.href = url;
    link.download = "tududu-" + assetId.slice(0, 8) + ".png";
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

        <div className="lightbox-actions">
          <button className="ghost" onClick={onClose}>
            关闭
          </button>
          <span className="spacer" />
          {onEdit && (
            <button className="ghost" onClick={onEdit}>
              以这张继续改
            </button>
          )}
          {onPublish && <button onClick={onPublish}>发布到广场</button>}
          <button className="ghost" onClick={download} disabled={!url}>
            下载
          </button>
        </div>
      </div>
    </div>
  );
}
