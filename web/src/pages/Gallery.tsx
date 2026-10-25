import { useEffect, useState } from "react";

import AuthImage from "../components/AuthImage";
import Lightbox from "../components/Lightbox";
import { request } from "../lib/api";
import type { Asset } from "../lib/types";

/** 「以图改图」时把选中的图存下来，对话页启动时取走。 */
const PENDING_KEY = "tududu.pendingInput";

export default function GalleryPage() {
  const [items, setItems] = useState<Asset[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [viewing, setViewing] = useState<Asset | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      try {
        const rows = await request<Asset[]>("/api/v1/assets?limit=60");
        if (!cancelled) setItems(rows);
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "加载失败");
        }
      }
    }

    void load();
    return () => {
      cancelled = true;
    };
  }, []);

  function editWith(assetId: string) {
    localStorage.setItem(PENDING_KEY, assetId);
    window.location.hash = "#/create";
  }

  return (
    <div className="page">
      <h2>我的相册</h2>
      {error && <p className="error">{error}</p>}
      {items.length === 0 && !error && <p className="hint">还没有图片</p>}

      <div className="grid">
        {items.map((item) => (
          <button key={item.id} className="image-tap" onClick={() => setViewing(item)}>
            <AuthImage assetId={item.id} alt={item.source} variant="thumb" />
          </button>
        ))}
      </div>

      {viewing && (
        <Lightbox
          assetId={viewing.id}
          onClose={() => setViewing(null)}
          onEdit={() => {
            const id = viewing.id;
            setViewing(null);
            editWith(id);
          }}
        />
      )}
    </div>
  );
}
