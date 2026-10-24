import { useCallback, useEffect, useState } from "react";

import AuthImage from "../components/AuthImage";
import { request } from "../lib/api";
import type { useAuth } from "../lib/store";
import type { Job, SessionDetail, TurnResult } from "../lib/types";

type Auth = ReturnType<typeof useAuth>;

export default function SessionDetailPage({
  auth,
  sessionId
}: {
  auth: Auth;
  sessionId: string;
}) {
  const [detail, setDetail] = useState<SessionDetail | null>(null);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const found = await request<SessionDetail>(`/api/v1/sessions/${sessionId}`);
      setDetail(found);
    } catch (err) {
      setError(err instanceof Error ? err.message : "加载会话失败");
    }
  }, [sessionId]);

  useEffect(() => {
    void load();
  }, [load]);

  async function send() {
    if (!text.trim() || !detail) return;

    setBusy(true);
    setError(null);

    try {
      const turn = await request<TurnResult>(`/api/v1/sessions/${sessionId}/messages`, {
        method: "POST",
        body: { text }
      });

      setText("");
      await load();
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
        if (latest.status === 2 || latest.status === 3) {
          await load();
          return;
        }
      } catch {
        return;
      }
    }
  }

  return (
    <div className="page">
      <a className="link back" href="#/me">
        返回
      </a>

      <h2>{detail?.session.title ?? "会话"}</h2>
      {error && <p className="error">{error}</p>}

      <div className="list">
        {detail?.messages.map((message) => (
          <article key={message.id} className="card">
            <p className="hint">{message.role === "user" ? "我" : "图嘟嘟"}</p>

            {message.content && <p className="prompt">{message.content}</p>}

            {message.input_asset_id && (
              <AuthImage assetId={message.input_asset_id} alt="输入图" />
            )}

            {message.output_asset_id && (
              <AuthImage assetId={message.output_asset_id} alt="结果图" />
            )}

            {message.status === 1 && <p className="hint">生成中…</p>}
            {message.status === 4 && <p className="error">这一步失败了</p>}
          </article>
        ))}
      </div>

      <section className="card">
        <textarea
          value={text}
          onChange={(event) => setText(event.target.value)}
          rows={3}
          placeholder="继续改，例如：人物再自然一点"
        />
        <button onClick={() => void send()} disabled={busy || !text.trim()}>
          {busy ? "提交中…" : "继续修改"}
        </button>
      </section>
    </div>
  );
}
