import { useEffect, useState } from "react";

import AuthImage from "../components/AuthImage";
import Lightbox from "../components/Lightbox";
import { request } from "../lib/api";
import type { PromptCategory, PromptItem } from "../lib/types";

interface LikeResult {
  liked: boolean;
  like_count: number;
}

export default function PlazaPage() {
  const [items, setItems] = useState<PromptItem[]>([]);
  const [categories, setCategories] = useState<PromptCategory[]>([]);
  const [active, setActive] = useState<number | null>(null);
  const [keyword, setKeyword] = useState("");
  const [submitted, setSubmitted] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [viewing, setViewing] = useState<PromptItem | null>(null);
  const [liked, setLiked] = useState<Record<string, boolean>>({});
  const [bump, setBump] = useState<string | null>(null);

  useEffect(() => {
    void request<PromptCategory[]>("/api/v1/prompt-categories", { auth: false })
      .then(setCategories)
      .catch(() => setCategories([]));
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      try {
        const params = new URLSearchParams();
        if (active) params.set("category_id", String(active));
        if (submitted) params.set("q", submitted);
        params.set("limit", "30");

        const rows = await request<PromptItem[]>("/api/v1/prompts?" + params.toString(), {
          auth: false
        });

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
  }, [active, submitted]);

  /// 做同款：把提示词带进新对话
  function useThis(promptText: string) {
    localStorage.setItem("tududu.pendingPrompt", promptText);
    window.location.hash = "#/";
  }

  async function toggleLike(item: PromptItem) {
    setBump(item.short_code);

    try {
      const result = await request<LikeResult>("/api/v1/prompts/" + item.short_code + "/like", {
        method: "POST"
      });

      setLiked((prev) => ({ ...prev, [item.short_code]: result.liked }));
      setItems((prev) =>
        prev.map((row) =>
          row.short_code === item.short_code ? { ...row, like_count: result.like_count } : row
        )
      );
    } catch {
      // 没登录时点赞会失败，静默忽略
    } finally {
      window.setTimeout(() => setBump(null), 420);
    }
  }

  return (
    <div className="page">
      <header className="page-head">
        <h2>广场</h2>
      </header>

      <form
        className="search"
        onSubmit={(event) => {
          event.preventDefault();
          setSubmitted(keyword.trim());
        }}
      >
        <input
          value={keyword}
          onChange={(event) => setKeyword(event.target.value)}
          placeholder="搜标题或提示词"
        />
        <button type="submit">搜索</button>
      </form>

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
      {items.length === 0 && !error && <p className="hint">没有找到作品</p>}

      <div className="plaza-list">
        {items.map((item, index) => (
          <article
            key={item.id}
            className="plaza-card"
            style={{ animationDelay: Math.min(index * 40, 480) + "ms" }}
          >
            {item.cover_asset_id && (
              <button
                className="plaza-cover"
                onClick={() => setViewing(item)}
                aria-label="查看作品"
              >
                <AuthImage assetId={item.cover_asset_id} alt={item.title} variant="thumb" />

                <span className="plaza-badge">{item.model}</span>
              </button>
            )}

            <div className="plaza-body">
              <h3>{item.title}</h3>
              <p className="prompt">{item.prompt_text}</p>

              <div className="plaza-actions">
                <button
                  className={liked[item.short_code] ? "like on" : "like"}
                  onClick={() => void toggleLike(item)}
                  aria-label="点赞"
                >
                  <span className={bump === item.short_code ? "heart pop" : "heart"}>♥</span>
                  {item.like_count}
                </button>

                <span className="hint">看 {item.view_count}</span>

                <span className="spacer" />

                <button className="chip accent-chip" onClick={() => useThis(item.prompt_text)}>
                  做同款
                </button>
              </div>
            </div>
          </article>
        ))}
      </div>

      {viewing?.cover_asset_id && (
        <Lightbox assetId={viewing.cover_asset_id} onClose={() => setViewing(null)} />
      )}
    </div>
  );
}
