import { useEffect, useMemo, useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { AdminModel, Provider } from "../lib/types";

export default function ModelsPage() {
  const models = useLoad(() => adminRequest<AdminModel[]>("/api/v1/admin/models"));
  const providers = useLoad(() => adminRequest<Provider[]>("/api/v1/admin/providers"));

  const [providerId, setProviderId] = useState("");
  const [code, setCode] = useState("");
  const [alias, setAlias] = useState("");
  const [note, setNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [keyword, setKeyword] = useState("");

  const list = models.data ?? [];
  const options = providers.data ?? [];
  const current = providerId || options[0]?.id || "";

  const visible = useMemo(() => {
    const needle = keyword.trim().toLowerCase();
    if (!needle) return list;
    return list.filter(
      (item) =>
        item.code.toLowerCase().includes(needle) ||
        item.display_name.toLowerCase().includes(needle) ||
        item.provider_name.toLowerCase().includes(needle)
    );
  }, [list, keyword]);

  const shown = list.filter((item) => item.enabled).length;

  async function create() {
    setBusy(true);
    setNote(null);

    try {
      await adminRequest("/api/v1/admin/models", {
        method: "POST",
        body: {
          provider_id: current,
          code: code.trim(),
          display_name: alias.trim() || code.trim()
        }
      });

      setCode("");
      setAlias("");
      setNote("已添加");
      await models.reload();
    } catch (err) {
      setNote(err instanceof Error ? err.message : "添加失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section>
      <header className="page-head">
        <div>
          <h2>模型</h2>
          <p className="hint">
            共 {list.length} 个，用户端可见 {shown} 个。别称就是用户看到的名字，排序小的在前。
          </p>
        </div>
        <button onClick={() => void models.reload()}>刷新</button>
      </header>

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
                模型代号
                <input
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                  placeholder="上游要求的 model 参数，例如 gpt-image-1"
                />
              </label>
              <label>
                别称（用户看到的名字）
                <input
                  value={alias}
                  onChange={(e) => setAlias(e.target.value)}
                  placeholder="留空则同代号"
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
        <div className="panel-head">
          <h3>全部模型</h3>
          <input
            className="search-input"
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            placeholder="搜代号、别称、渠道商"
          />
        </div>

        <table>
          <thead>
            <tr>
              <th>模型代号</th>
              <th>别称</th>
              <th>渠道商</th>
              <th>排序</th>
              <th>用户可见</th>
              <th>备注</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {visible.map((item) => (
              <ModelRow key={item.id} item={item} onSaved={() => void models.reload()} />
            ))}
          </tbody>
        </table>

        {visible.length === 0 && (
          <p className="hint">
            {list.length === 0 ? "还没有模型，可以从上游刷新，也可以在上面手工加。" : "没有匹配的模型。"}
          </p>
        )}
      </div>
    </section>
  );
}

function ModelRow({ item, onSaved }: { item: AdminModel; onSaved: () => void }) {
  const [alias, setAlias] = useState(item.display_name);
  const [sort, setSort] = useState(String(item.sort_order));
  const [remark, setRemark] = useState(item.remark);
  const [enabled, setEnabled] = useState(item.enabled);
  const [busy, setBusy] = useState(false);
  const [tip, setTip] = useState<string | null>(null);

  useEffect(() => {
    setAlias(item.display_name);
    setSort(String(item.sort_order));
    setRemark(item.remark);
    setEnabled(item.enabled);
  }, [item.display_name, item.sort_order, item.remark, item.enabled]);

  const dirty =
    alias !== item.display_name ||
    sort !== String(item.sort_order) ||
    remark !== item.remark ||
    enabled !== item.enabled;

  async function save() {
    setBusy(true);
    setTip(null);

    try {
      await adminRequest("/api/v1/admin/models/" + item.id, {
        method: "PUT",
        body: {
          display_name: alias.trim() || item.code,
          enabled,
          sort_order: Number(sort) || 100,
          remark: remark.trim()
        }
      });

      setTip("已保存");
      onSaved();
    } catch (err) {
      setTip(err instanceof Error ? err.message : "保存失败");
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (!window.confirm("删除后用户端立刻看不到这个模型，确定吗？")) return;

    setBusy(true);

    try {
      await adminRequest("/api/v1/admin/models/" + item.id, { method: "DELETE" });
      onSaved();
    } catch (err) {
      setTip(err instanceof Error ? err.message : "删除失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <tr className={enabled ? "" : "muted"}>
      <td className="mono">{item.code}</td>
      <td>
        <input
          className="cell-input"
          value={alias}
          onChange={(e) => setAlias(e.target.value)}
          placeholder={item.code}
        />
      </td>
      <td>{item.provider_name}</td>
      <td>
        <input
          className="cell-input narrow"
          value={sort}
          onChange={(e) => setSort(e.target.value)}
          inputMode="numeric"
        />
      </td>
      <td>
        <label className="switch">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
          />
          <span>{enabled ? "显示" : "隐藏"}</span>
        </label>
      </td>
      <td>
        <input
          className="cell-input"
          value={remark}
          onChange={(e) => setRemark(e.target.value)}
          placeholder="给内部看的备注"
        />
      </td>
      <td className="actions">
        <button className="small" onClick={() => void save()} disabled={busy || !dirty}>
          {busy ? "…" : "保存"}
        </button>
        <button className="small danger" onClick={() => void remove()} disabled={busy}>
          删除
        </button>
        {tip && <span className="hint">{tip}</span>}
      </td>
    </tr>
  );
}
