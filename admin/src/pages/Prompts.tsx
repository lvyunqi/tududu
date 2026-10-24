import { useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { AdminPrompt, SensitiveWord } from "../lib/types";

export default function PromptsPage() {
  const [status, setStatus] = useState(2);
  const { data, error, loading, reload } = useLoad(() =>
    adminRequest<AdminPrompt[]>(`/api/v1/admin/prompts?status=${status}&limit=50`),
  );

  async function setState(item: AdminPrompt, next: number) {
    await adminRequest(`/api/v1/admin/prompts/${item.short_code}/status`, {
      method: "POST",
      body: { status: next }
    });
    await reload();
  }

  return (
    <section>
      <h2>广场作品</h2>
      <div className="chips">
        <button
          className={status === 1 ? "chip active" : "chip"}
          onClick={() => setStatus(1)}
        >
          待审核
        </button>
        <button
          className={status === 2 ? "chip active" : "chip"}
          onClick={() => setStatus(2)}
        >
          已发布
        </button>
        <button
          className={status === 3 ? "chip active" : "chip"}
          onClick={() => setStatus(3)}
        >
          已下架
        </button>
      </div>

      <WordPanel />

      {loading && <p className="hint">加载中…</p>}
      {error && <p className="error">{error}</p>}

      <table>
        <thead>
          <tr>
            <th>编号</th>
            <th>标题</th>
            <th>模型</th>
            <th>赞</th>
            <th>浏览</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {(data ?? []).map((item) => (
            <tr key={item.id}>
              <td className="mono">{item.short_code}</td>
              <td>
                {item.title}
                <div className="hint">{item.prompt_text.slice(0, 40)}</div>
              </td>
              <td>{item.model}</td>
              <td className="mono">{item.like_count}</td>
              <td className="mono">{item.view_count}</td>
              <td className="actions">
                {status !== 2 && (
                  <button className="small" onClick={() => void setState(item, 2)}>
                    通过
                  </button>
                )}
                {status !== 3 && (
                  <button className="small" onClick={() => void setState(item, 3)}>
                    下架
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {(data ?? []).length === 0 && !loading && <p className="hint">没有作品</p>}
    </section>
  );
}

function WordPanel() {
  const words = useLoad(() => adminRequest<SensitiveWord[]>("/api/v1/admin/sensitive-words"));
  const [draft, setDraft] = useState("");
  const [note, setNote] = useState<string | null>(null);

  async function add() {
    if (!draft.trim()) return;
    setNote(null);

    try {
      await adminRequest("/api/v1/admin/sensitive-words", {
        method: "POST",
        body: { word: draft.trim() }
      });
      setDraft("");
      await words.reload();
    } catch (err) {
      setNote(err instanceof Error ? err.message : "添加失败");
    }
  }

  async function remove(id: number) {
    await adminRequest(`/api/v1/admin/sensitive-words/${id}`, { method: "DELETE" });
    await words.reload();
  }

  return (
    <div className="panel">
      <h3>敏感词</h3>
      <p className="hint">命中的作品会转成待审核，不会直接上架。</p>
      <div className="form-row">
        <input
          placeholder="要拦截的词"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
        <button onClick={() => void add()} disabled={!draft.trim()}>
          添加
        </button>
      </div>
      {note && <p className="error">{note}</p>}
      <div className="chips">
        {(words.data ?? []).map((item) => (
          <button key={item.id} className="chip" onClick={() => void remove(item.id)}>
            {item.word} ×
          </button>
        ))}
      </div>
      {(words.data ?? []).length === 0 && <p className="hint">还没有词条</p>}
    </div>
  );
}
