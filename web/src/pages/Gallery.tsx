import { useEffect, useState } from "react";

import AuthImage from "../components/AuthImage";
import { request } from "../lib/api";
import type { Asset } from "../lib/types";

export default function GalleryPage() {
  const [items, setItems] = useState<Asset[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      try {
        const rows = await request<Asset[]>("/api/v1/assets?limit=30");
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

  return (
    <div className="page">
      <h2>我的相册</h2>
      {error && <p className="error">{error}</p>}
      {items.length === 0 && !error && <p className="hint">还没有图片</p>}

      <div className="grid">
        {items.map((item) => (
          <AuthImage key={item.id} assetId={item.id} alt={item.source} variant="thumb" />
        ))}
      </div>
    </div>
  );
}
