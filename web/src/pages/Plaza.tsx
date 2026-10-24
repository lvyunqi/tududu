import { useEffect, useState } from "react";

import { request } from "../lib/api";
import type { PromptCategory, PromptItem } from "../lib/types";

export default function PlazaPage() {
  const [items, setItems] = useState<PromptItem[]>([]);
  const [categories, setCategories] = useState<PromptCategory[]>([]);
  const [active, setActive] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void request<PromptCategory[]>("/api/v1/prompt-categories", { auth: false })
      .then(setCategories)
      .catch(() => setCategories([]));
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      try {
        const query = active ? `?category_id=${active}&limit=30` : "?limit=30";
        const rows = await request<PromptItem[]>(`/api/v1/prompts${query}`, { auth: false });
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
  }, [active]);

  return (
    <div className="page">
      <h2>提示词广场</h2>

      <div className="chips">
        <button className={active === null ? "chip active" : "chip"} onClick={() => setActive(null)}>
          全部
        </button>
        {categories.map((item) => (
          <button
            key={item.id}
            className={active === item.id ? "chip active" : "chip"}
            onClick={() => setActive(item.id)}
          >
            {item.name}
          </button>
        ))}
      </div>

      {error && <p className="error">{error}</p>}
      {items.length === 0 && !error && <p className="hint">还没有作品</p>}

      <div className="list">
        {items.map((item) => (
          <article key={item.id} className="card">
            <h3>{item.title}</h3>
            <p className="prompt">{item.prompt_text}</p>
            <p className="hint">
              {item.model} · 赞 {item.like_count} · 看 {item.view_count}
            </p>
          </article>
        ))}
      </div>
    </div>
  );
}
