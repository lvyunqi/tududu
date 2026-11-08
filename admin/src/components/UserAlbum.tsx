import { useEffect, useState } from "react";

import { useLoad } from "./useLoad";
import { adminRequest, adminBlob } from "../lib/api";
import type { AdminAsset } from "../lib/types";

/** 缩略图要带管理员令牌才取得到，先取 blob 再显示。 */
function AssetThumb({ id }: { id: string }) {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    let revoked = false;
    let objectUrl: string | null = null;

    void adminBlob("/api/v1/assets/" + id + "/file?size=thumb")
      .then((blob) => {
        objectUrl = URL.createObjectURL(blob);
        if (!revoked) setUrl(objectUrl);
      })
      .catch(() => setUrl(null));

    return () => {
      revoked = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [id]);

  if (!url) return <div className="album-ph" />;

  return <img className="album-img" src={url} alt="" loading="lazy" />;
}

const STATUS_LABEL: Record<number, string> = {
  1: "正常",
  2: "回收站",
  3: "已封禁"
};

/** 用户相册：管理员可以看、可以封禁、可以删。 */
export default function UserAlbum({
  userId,
  nickname,
  onClose
}: {
  userId: string;
  nickname: string;
  onClose: () => void;
}) {
  const { data, loading, reload } = useLoad(() =>
    adminRequest<AdminAsset[]>("/api/v1/admin/users/" + userId + "/assets?limit=60")
  );
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  async function ban(id: string, banned: boolean) {
    setBusy(true);
    setNote(null);

    try {
      await adminRequest("/api/v1/admin/assets/ban", {
        method: "POST",
        body: { ids: [id], banned }
      });
      await reload();
    } catch (err) {
      setNote(err instanceof Error ? err.message : "操作失败");
    } finally {
      setBusy(false);
    }
  }

  async function remove(id: string) {
    if (!window.confirm("删除后进入回收站，确定吗？")) return;

    setBusy(true);
    setNote(null);

    try {
      await adminRequest("/api/v1/admin/assets/" + id, { method: "DELETE" });
      await reload();
    } catch (err) {
      setNote(err instanceof Error ? err.message : "删除失败");
    } finally {
      setBusy(false);
    }
  }

  const items = data ?? [];

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal album" onClick={(event) => event.stopPropagation()}>
        <header className="modal-head">
          <strong>{nickname} 的相册</strong>
          <span className="hint">
            {items.length} 张 · 封禁后用户端立刻看不到，也下载不了
          </span>
          <button className="small" onClick={onClose}>
            关闭
          </button>
        </header>

        {note && <p className="hint">{note}</p>}
        {loading && <p className="hint">加载中…</p>}
        {!loading && items.length === 0 && <p className="hint">这个用户还没有图片</p>}

        <div className="album-grid">
          {items.map((item) => (
            <figure key={item.id} className={item.status === 3 ? "album-cell banned" : "album-cell"}>
              <AssetThumb id={item.id} />
              <figcaption>
                <span className="mono">{STATUS_LABEL[item.status] ?? item.status}</span>
                <span className="hint">{item.source}</span>
              </figcaption>
              <div className="album-actions">
                {item.status === 3 ? (
                  <button className="small" disabled={busy} onClick={() => void ban(item.id, false)}>
                    解封
                  </button>
                ) : (
                  <button className="small danger" disabled={busy} onClick={() => void ban(item.id, true)}>
                    封禁
                  </button>
                )}
                <button className="small" disabled={busy || item.status === 2} onClick={() => void remove(item.id)}>
                  删除
                </button>
              </div>
            </figure>
          ))}
        </div>
      </div>
    </div>
  );
}
