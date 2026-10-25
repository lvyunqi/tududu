import { useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { AdminModel, Provider } from "../lib/types";

export default function ModelsPage() {
  const models = useLoad(() => adminRequest<AdminModel[]>("/api/v1/admin/models"));
  const providers = useLoad(() => adminRequest<Provider[]>("/api/v1/admin/providers"));

  const [providerId, setProviderId] = useState("");
  const [code, setCode] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [note, setNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const list = models.data ?? [];

  async function create() {
    setBusy(true);
    setNote(null);

    try {
      await adminRequest("/api/v1/admin/models", {
        method: "POST",
        body: {
          provider_id: providerId,
          code: code.trim(),
          display_name: displayName.trim() || code.trim()
        }
      });

      setCode("");
      setDisplayName("");
      setNote("已添加");
      await models.reload();
    } catch (err) {
      setNote(err instanceof Error ? err.message : "添加失败");
    } finally {
      setBusy(false);
    }
  }

  async function toggle(item: AdminModel) {
    await adminRequest("/api/v1/admin/models/" + item.id, {
      method: "PATCH",
      body: { enabled: !item.enabled }
    });
    await models.reload();
  }

  const options = providers.data ?? [];
  const current = providerId || options[0]?.id || "";

  return (
    <section>
      <header className="page-head">
        <h2>模型</h2>
        <button onClick={() => void models.reload()}>刷新</button>
      </header>

      <p className="hint">
        用户端只能选这里启用的模型。停用之后它就不再出现在对话页的模型列表里，已有会话不受影响。
      </p>

      <div className="panel">
        <h3>添加模型</h3>

        {options.length === 0 ? (
          <p className="hint">还没有渠道商，先去「渠道」页添加。</p>
        ) : (
          <>
            <div className="form-grid">
              <label>
                所属渠道商
                <select value={current} onChange={(e) => setProviderId(e.target.value)}>
                  {options.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                模型代码
                <input
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                  placeholder="例如：gpt-image-1"
                />
              </label>
              <label>
                显示名（可不填）
                <input
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  placeholder="例如：写实修图"
                />
              </label>
            </div>

            {note && <p className="hint">{note}</p>}

            <button onClick={() => void create()} disabled={busy || !current || !code.trim()}>
              {busy ? "提交中…" : "添加"}
            </button>
          </>
        )}
      </div>

      <div className="panel">
        <h3>全部模型</h3>
        <table>
          <thead>
            <tr>
              <th>模型代码</th>
              <th>显示名</th>
              <th>渠道商</th>
              <th>状态</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            {list.map((item) => (
              <tr key={item.id}>
                <td className="mono">{item.code}</td>
                <td>{item.display_name}</td>
                <td>{item.provider_name}</td>
                <td>{item.enabled ? "启用" : "停用"}</td>
                <td className="actions">
                  <button className="small" onClick={() => void toggle(item)}>
                    {item.enabled ? "停用" : "启用"}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {list.length === 0 && (
          <p className="hint">还没有模型。可以从上游刷新，也可以在上面的表单里手工加。</p>
        )}
      </div>
    </section>
  );
}
