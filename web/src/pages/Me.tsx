import { useEffect, useState } from "react";

import Toast from "../components/Toast";
import { BASE, request } from "../lib/api";
import type { useAuth } from "../lib/store";
import type { PromptItem, Session } from "../lib/types";

type Auth = ReturnType<typeof useAuth>;

export default function MePage({ auth }: { auth: Auth }) {
  const [works, setWorks] = useState<PromptItem[]>([]);
  const [sessions, setSessions] = useState<Session[]>([]);
  const [toast, setToast] = useState<string | null>(null);

  async function loadWorks() {
    try {
      const rows = await request<PromptItem[]>("/api/v1/my/prompts?limit=50");
      setWorks(rows);
    } catch {
      setWorks([]);
    }
  }

  useEffect(() => {
    void loadWorks();

    void request<Session[]>("/api/v1/sessions?limit=20")
      .then(setSessions)
      .catch(() => setSessions([]));
  }, []);

  return (
    <div className="page">
      <header className="page-head">
        <h2>{auth.user?.nickname}</h2>
        <span className="tag">编号 {auth.user?.short_code}</span>
      </header>

      <section className="card">
        <span className="label">余额</span>
        <p className="balance-line">
          <strong>{auth.balance}</strong>
          <span>{auth.unitName}</span>
        </p>
        <a className="button" href="#/recharge">
          去充值
        </a>
      </section>

      <section className="card">
        <div className="card-head">
          <h3>我的对话</h3>
          <a className="chip" href="#/">
            新对话
          </a>
        </div>

        {sessions.length === 0 && <p className="hint">还没有对话</p>}
        {sessions.map((item) => (
          <a key={item.id} className="row" href={"#/session/" + item.id}>
            <span className="row-main">{item.title || "未命名对话"}</span>
            <span className="hint">{formatTime(item.last_active_at)}</span>
          </a>
        ))}
      </section>

      <section className="card">
        <div className="card-head">
          <h3>我发布的作品</h3>
          <span className="tag">{works.length} 个</span>
        </div>

        {works.length === 0 && <p className="hint">还没有发布过</p>}

        {works.map((item) => (
          <WorkRow
            key={item.id}
            item={item}
            onToast={setToast}
            onRemoved={(id) => setWorks((prev) => prev.filter((row) => row.id !== id))}
            onChanged={() => void loadWorks()}
          />
        ))}
      </section>

      <section className="card">
        <h3>调试信息</h3>
        <p className="hint">接口地址：{BASE || "当前站点同源"}</p>
        <p className="hint">版本：{import.meta.env.MODE}</p>
      </section>

      <button className="danger" onClick={() => void auth.logout()}>
        退出登录
      </button>

      {toast && <Toast text={toast} onDone={() => setToast(null)} />}
    </div>
  );
}

/** 一行作品。删除要点两下：先展开确认，再真的删，删除时整行收起来。 */
function WorkRow({
  item,
  onToast,
  onRemoved,
  onChanged
}: {
  item: PromptItem;
  onToast: (text: string) => void;
  onRemoved: (id: string) => void;
  onChanged: () => void;
}) {
  const [stage, setStage] = useState<"idle" | "confirm" | "busy" | "leaving">("idle");

  async function hide() {
    setStage("busy");

    try {
      await request("/api/v1/prompts/" + item.short_code, { method: "DELETE" });
      onToast("已从广场撤下");
      onChanged();
      setStage("idle");
    } catch (err) {
      onToast(err instanceof Error ? err.message : "操作失败");
      setStage("idle");
    }
  }

  async function purge() {
    setStage("busy");

    try {
      await request("/api/v1/prompts/" + item.short_code + "/permanent", {
        method: "DELETE"
      });

      // 先播收起动画，再真的从列表里去掉
      setStage("leaving");
      window.setTimeout(() => onRemoved(item.id), 260);
      onToast("作品已删除");
    } catch (err) {
      onToast(err instanceof Error ? err.message : "删除失败");
      setStage("idle");
    }
  }

  const leaving = stage === "leaving";
  const busy = stage === "busy";

  return (
    <div className={leaving ? "row work-row leaving" : "row work-row"}>
      <span className="row-main">{item.title}</span>

      <span className={item.status === 2 ? "tag ok" : "tag"}>
        {item.status === 2 ? "广场可见" : item.status === 1 ? "待审核" : "已隐藏"}
      </span>

      <span className="row-actions">
        {stage === "confirm" ? (
          <>
            <button className="mini" onClick={() => setStage("idle")} disabled={busy}>
              取消
            </button>
            <button className="mini danger" onClick={() => void purge()} disabled={busy}>
              {busy ? "删除中" : "确认删除"}
            </button>
          </>
        ) : (
          <>
            {item.status === 2 && (
              <button className="mini" onClick={() => void hide()} disabled={busy}>
                {busy ? "处理中" : "隐藏"}
              </button>
            )}
            <button className="mini danger" onClick={() => setStage("confirm")} disabled={busy}>
              删除
            </button>
          </>
        )}
      </span>
    </div>
  );
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";

  const diff = Date.now() - date.getTime();
  if (diff < 60_000) return "刚刚";
  if (diff < 3_600_000) return Math.floor(diff / 60_000) + " 分钟前";
  if (diff < 86_400_000) return Math.floor(diff / 3_600_000) + " 小时前";

  return date.getMonth() + 1 + "月" + date.getDate() + "日";
}
