import { useEffect, useRef, useState } from "react";

import AuthImage from "../components/AuthImage";
import Lightbox from "../components/Lightbox";
import Toast from "../components/Toast";
import { request } from "../lib/api";
import type { Asset } from "../lib/types";

/** 「以图改图」时把选中的图存下来，对话页启动时取走。 */
const PENDING_KEY = "tududu.pendingInput";

export default function GalleryPage() {
  const [items, setItems] = useState<Asset[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [viewing, setViewing] = useState<number | null>(null);
  const [selecting, setSelecting] = useState(false);
  const [picked, setPicked] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  const longPress = useRef<number | null>(null);

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

  function tap(item: Asset, index: number) {
    if (!selecting) {
      setViewing(index);
      return;
    }

    setPicked((prev) =>
      prev.includes(item.id) ? prev.filter((x) => x !== item.id) : [...prev, item.id]
    );
  }

  /// 长按任意一张进多选，不用先去点右上角
  function pressStart(item: Asset) {
    longPress.current = window.setTimeout(() => {
      setSelecting(true);
      setPicked((prev) => (prev.includes(item.id) ? prev : [...prev, item.id]));
    }, 420);
  }

  function pressEnd() {
    if (longPress.current) {
      window.clearTimeout(longPress.current);
      longPress.current = null;
    }
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

      setNote("已移入回收站 " + picked.length + " 张");
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

  function step(delta: number) {
    if (viewing === null) return;

    const next = viewing + delta;
    if (next < 0 || next >= items.length) return;

    setViewing(next);
  }

  return (
    <div className={selecting ? "page selecting" : "page"}>
      <header className="page-head">
        <a className="back-home" href="#/" aria-label="回到对话">
          ←
        </a>
        <div className="page-head-main">
          <h2>相册</h2>
          <p className="hint">{items.length} 张图片</p>
        </div>

        {selecting ? (
          <button
            className="chip"
            onClick={() => {
              setSelecting(false);
              setPicked([]);
            }}
          >
            完成
          </button>
        ) : (
          <button className="chip" onClick={() => setSelecting(true)} disabled={items.length === 0}>
            选择
          </button>
        )}
      </header>

      {error && <p className="error">{error}</p>}

      {items.length === 0 && !error && (
        <div className="empty">
          <div className="empty-mark" aria-hidden="true" />
          <p className="hint">还没有图片，去对话页生成一张吧</p>
        </div>
      )}

      <div className={selecting ? "grid picking" : "grid"}>
        {items.map((item, index) => {
          const active = picked.includes(item.id);

          return (
            <button
              key={item.id}
              className={active ? "tile picked" : "tile"}
              style={{ animationDelay: Math.min(index * 28, 420) + "ms" }}
              onPointerDown={() => pressStart(item)}
              onPointerUp={pressEnd}
              onPointerLeave={pressEnd}
              onContextMenu={(event) => event.preventDefault()}
              onClick={() => tap(item, index)}
            >
              <AuthImage assetId={item.id} alt={item.source} variant="thumb" />
              {selecting && (
                <span className={active ? "tick on" : "tick"}>{active ? "✓" : ""}</span>
              )}
            </button>
          );
        })}
      </div>

      {selecting && (
        <div className="action-bar">
          <button
            className="chip"
            onClick={() => setPicked(picked.length === items.length ? [] : items.map((i) => i.id))}
          >
            {picked.length === items.length ? "取消全选" : "全选"}
          </button>

          <span className="action-count">已选 {picked.length}</span>

          <button
            className="chip danger-chip"
            onClick={() => void removePicked()}
            disabled={busy || picked.length === 0}
          >
            {busy ? "删除中…" : "删除"}
          </button>
        </div>
      )}

      {viewing !== null && items[viewing] && (
        <Lightbox
          assetId={items[viewing].id}
          onClose={() => setViewing(null)}
          onPrev={() => step(-1)}
          onNext={() => step(1)}
          position={{ index: viewing, total: items.length }}
          onEdit={() => {
            const id = items[viewing].id;
            setViewing(null);
            editWith(id);
          }}
        />
      )}

      {note && <Toast text={note} onDone={() => setNote(null)} />}
    </div>
  );
}
