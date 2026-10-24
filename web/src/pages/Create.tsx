import { useEffect, useState } from "react";

import AuthImage from "../components/AuthImage";
import { request } from "../lib/api";
import type { useAuth } from "../lib/store";
import type {
  Asset,
  Job,
  PromptCategory,
  PromptItem,
  Session,
  TurnResult
} from "../lib/types";

type Auth = ReturnType<typeof useAuth>;

const MODEL = "gpt-image-1";

export default function CreatePage({ auth }: { auth: Auth }) {
  const [asset, setAsset] = useState<Asset | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const [text, setText] = useState("");
  const [job, setJob] = useState<Job | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [categories, setCategories] = useState<PromptCategory[]>([]);
  const [title, setTitle] = useState("");
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [published, setPublished] = useState<PromptItem | null>(null);
  const [publishError, setPublishError] = useState<string | null>(null);
  const [publishing, setPublishing] = useState(false);

  useEffect(() => {
    void request<PromptCategory[]>("/api/v1/prompt-categories", { auth: false })
      .then(setCategories)
      .catch(() => setCategories([]));
  }, []);

  async function upload(file: File) {
    setBusy(true);
    setError(null);

    try {
      const form = new FormData();
      form.append("file", file);
      const created = await request<Asset>("/api/v1/uploads", { method: "POST", form });
      setAsset(created);
    } catch (err) {
      setError(err instanceof Error ? err.message : "上传失败");
    } finally {
      setBusy(false);
    }
  }

  async function submit() {
    if (!text.trim()) return;

    setBusy(true);
    setError(null);
    setPublished(null);

    try {
      let current = session;
      if (!current) {
        current = await request<Session>("/api/v1/sessions", {
          method: "POST",
          body: { model: MODEL, title: text.slice(0, 20) }
        });
        setSession(current);
      }

      const turn = await request<TurnResult>(`/api/v1/sessions/${current.id}/messages`, {
        method: "POST",
        body: { text, input_asset_id: asset?.id ?? null }
      });

      setJob({ id: turn.job_id, status: 0, output_asset_id: null, error_message: null, price: 0 });
      await auth.refresh();
      void poll(turn.job_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "提交失败");
    } finally {
      setBusy(false);
    }
  }

  async function poll(jobId: string) {
    for (let i = 0; i < 60; i += 1) {
      await new Promise((resolve) => setTimeout(resolve, 3000));

      try {
        const latest = await request<Job>(`/api/v1/jobs/${jobId}`);
        setJob(latest);
        if (latest.status === 2 || latest.status === 3) {
          void auth.refresh();
          return;
        }
      } catch {
        return;
      }
    }
  }

  async function publish() {
    if (!job?.output_asset_id) return;

    setPublishing(true);
    setPublishError(null);

    try {
      const finalTitle = title.trim() || text.slice(0, 24);
      const created = await request<PromptItem>("/api/v1/prompts", {
        method: "POST",
        body: {
          title: finalTitle,
          prompt_text: text,
          model: MODEL,
          category_id: categoryId,
          cover_asset_id: job.output_asset_id,
          source_session_id: session?.id ?? null
        }
      });
      setPublished(created);
    } catch (err) {
      setPublishError(err instanceof Error ? err.message : "发布失败");
    } finally {
      setPublishing(false);
    }
  }

  const done = job?.status === 2 && job.output_asset_id;

  return (
    <div className="page">
      <section className="card">
        <h2>① 选一张图</h2>
        <input
          type="file"
          accept="image/png,image/jpeg,image/webp"
          onChange={(event) => {
            const file = event.target.files?.[0];
            if (file) void upload(file);
          }}
        />
        {asset && <p className="hint">已上传：{asset.id.slice(0, 8)}</p>}
      </section>

      <section className="card">
        <h2>② 说清楚要改什么</h2>
        <textarea
          value={text}
          onChange={(event) => setText(event.target.value)}
          rows={4}
          placeholder="例如：把背景换成海边日落，人物光影保持一致"
        />
        <button onClick={() => void submit()} disabled={busy || !text.trim()}>
          {busy ? "提交中…" : "开始生成"}
        </button>
      </section>

      {error && <p className="error">{error}</p>}

      {job && (
        <section className="card">
          <h2>③ 结果</h2>
          <p className="hint">状态：{statusText(job.status)}</p>
          {job.error_message && <p className="error">{job.error_message}</p>}

          <div className="compare">
            <div>
              <p className="hint">原图</p>
              {asset ? (
                <AuthImage assetId={asset.id} alt="原图" />
              ) : (
                <div className="thumb placeholder" />
              )}
            </div>
            <div>
              <p className="hint">结果</p>
              {job.output_asset_id ? (
                <AuthImage assetId={job.output_asset_id} alt="结果图" />
              ) : (
                <div className="thumb placeholder" />
              )}
            </div>
          </div>
        </section>
      )}

      {done && !published && (
        <section className="card">
          <h2>④ 发布到广场</h2>
          <input
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder="给作品起个标题"
          />
          <div className="chips">
            {categories.map((item) => (
              <button
                key={item.id}
                className={categoryId === item.id ? "chip active" : "chip"}
                onClick={() => setCategoryId(item.id)}
              >
                {item.name}
              </button>
            ))}
          </div>
          {publishError && <p className="error">{publishError}</p>}
          <button onClick={() => void publish()} disabled={publishing}>
            {publishing ? "发布中…" : "发布"}
          </button>
        </section>
      )}

      {published && (
        <section className="card">
          <h2>已发布</h2>
          <p className="hint">作品编号 {published.short_code}</p>
        </section>
      )}
    </div>
  );
}

function statusText(status: number): string {
  switch (status) {
    case 0:
      return "排队中";
    case 1:
      return "生成中";
    case 2:
      return "已完成";
    case 3:
      return "失败";
    default:
      return "未知";
  }
}
