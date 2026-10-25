import { useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { SettingItem } from "../lib/types";

export default function SettingsPage() {
  const settings = useLoad(() =>
    adminRequest<SettingItem[]>("/api/v1/admin/settings"),
  );

  return (
    <section>
      <header className="page-head">
        <h2>运行时配置</h2>
        <button onClick={() => void settings.reload()}>刷新</button>
      </header>

      {settings.error && <p className="error">{settings.error}</p>}

      <p className="hint">
        这些改动保存后立刻生效，不需要重启。数据库、签名密钥、监听地址属于启动期参数，改不了，也不在这里。
      </p>

      {(settings.data ?? []).map((item) => (
        <SettingRow key={item.key} item={item} onSaved={() => void settings.reload()} />
      ))}

      {(settings.data ?? []).length === 0 && <p className="hint">没有可配置项</p>}
    </section>
  );
}

function SettingRow({ item, onSaved }: { item: SettingItem; onSaved: () => void }) {
  const [draft, setDraft] = useState(item.value);
  const [note, setNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function save() {
    setBusy(true);
    setNote(null);

    try {
      await adminRequest(`/api/v1/admin/settings/${item.key}`, {
        method: "PUT",
        body: { value: draft }
      });
      setNote("已保存");
      onSaved();
    } catch (err) {
      setNote(err instanceof Error ? err.message : "保存失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="setting-row">
      <div className="setting-meta">
        <code>{item.key}</code>
        <span className="hint">{item.description}</span>
      </div>

      <div className="setting-edit">
        <input value={draft} onChange={(event) => setDraft(event.target.value)} />
        <button onClick={() => void save()} disabled={busy || draft === item.value}>
          保存
        </button>
      </div>

      <span className={item.from_db ? "tag db" : "tag"}>
        {item.from_db ? "数据库" : "环境变量或默认"}
      </span>

      {note && <span className="hint">{note}</span>}
    </div>
  );
}
