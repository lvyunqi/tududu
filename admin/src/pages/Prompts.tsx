import { useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { AdminPrompt } from "../lib/types";

export default function PromptsPage() {
  const [status, setStatus] = useState(2);
  const { data, error, loading, reload } = useLoad(() =>
    adminRequest<AdminPrompt[]>(`/api/v1/admin/prompts?status=${status}&limit=50`),
  );

  async function toggle(item: AdminPrompt) {
    const next = status === 2 ? 3 : 2;
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
              <td>
                <button className="small" onClick={() => void toggle(item)}>
                  {status === 2 ? "下架" : "恢复"}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {(data ?? []).length === 0 && !loading && <p className="hint">没有作品</p>}
    </section>
  );
}
