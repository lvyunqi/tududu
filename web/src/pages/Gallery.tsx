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
  const [selecting, setSelecting] = useState(false);
  const [picked, setPicked] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);

  async function reload() {
    try {
      const rows = await request<Asset[]>("/api/v1/assets?limit=60");
      setItems(rows);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "加载失败");
    }
  }

  useEffect(() => {
    void reload();
  }, []);

  function tap(item: Asset) {
    if (!selecting) {
      setViewing(item);
      return;
    }

    setPicked((prev) =>
      prev.includes(item.id) ? prev.filter((x) => x !== item.id) : [...prev, item.id]
    );
  }

  async function removePicked() {
    if (picked.length === 0 || busy) return;

    setBusy(true);
    setError(null);

    try {
      await request("/api/v1/storage/batch-delete", {
        method: "POST",
        body: { ids: picked }
      });

      setPicked([]);
      setSelecting(false);
      await reload();
    } catch (err) {
      setError(err instanceof Error ? err.message : "删除失败");
    } finally {
      setBusy(false);
    }
  }

  function editWith(assetId: string) {
    localStorage.setItem(PENDING_KEY, assetId);
    window.location.hash = "#/create";
  }

  return (
    <div className="page">
      <header className="page-head">
        <h2>相册</h2>

        {selecting ? (
          <div className="head-actions">
            <button
              className="chip"
              onClick={() =>
                setPicked(picked.length === items.length ? [] : items.map((item) => item.id))
              }
            >
              {picked.length === items.length ? "取消全选" : "全选"}
            </button>
            <button
              className="chip danger-chip"
              onClick={() => void removePicked()}
              disabled={busy || picked.length === 0}
            >
              删除{picked.length > 0 ? " " + picked.length : ""}
            </button>
            <button
              className="chip"
              onClick={() => {
                setSelecting(false);
                setPicked([]);
              }}
            >
              完成
            </button>
          </div>
        ) : (
          <button className="chip" onClick={() => setSelecting(true)} disabled={items.length === 0}>
            选择
          </button>
        )}
      </header>

      {error && <p className="error">{error}</p>}
      {items.length === 0 && !error && <p className="hint">还没有图片</p>}

      <div className="grid">
        {items.map((item) => {
          const active = picked.includes(item.id);

          return (
            <button
              key={item.id}
              className={active ? "tile picked" : "tile"}
              onClick={() => tap(item)}
            >
              <AuthImage assetId={item.id} alt={item.source} variant="thumb" />
              {selecting && (
                <span className={active ? "tick on" : "tick"}>{active ? "✓" : ""}</span>
              )}
            </button>
          );
        })}
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
