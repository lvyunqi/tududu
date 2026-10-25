import { useEffect, useState } from "react";

import { BASE, request } from "../lib/api";
import type { useAuth } from "../lib/store";
import type { PromptItem, Session } from "../lib/types";

type Auth = ReturnType<typeof useAuth>;

export default function MePage({ auth }: { auth: Auth }) {
  const [works, setWorks] = useState<PromptItem[]>([]);
  const [sessions, setSessions] = useState<Session[]>([]);

  useEffect(() => {
    void request<PromptItem[]>("/api/v1/my/prompts?limit=20")
      .then(setWorks)
      .catch(() => setWorks([]));

    void request<Session[]>("/api/v1/sessions?limit=20")
      .then(setSessions)
      .catch(() => setSessions([]));
  }, []);

  return (
    <div className="page">
      <h2>{auth.user?.nickname}</h2>
      <p className="hint">编号 {auth.user?.short_code}</p>

      <section className="card">
        <h3>余额</h3>
        <p className="balance-line">
          {auth.balance} {auth.unitName}
        </p>
        <a className="button" href="#/recharge">
          去充值
        </a>
      </section>

      <section className="card">
        <h3>我的会话</h3>
        <a className="chip" href="#/" style={{ width: "fit-content" }}>
          开始新的对话
        </a>
        {sessions.length === 0 && <p className="hint">还没有创作记录</p>}
        {sessions.map((item) => (
          <a key={item.id} className="row" href={`#/session/${item.id}`}>
            <span>{item.title}</span>
            <span className="hint">{formatTime(item.last_active_at)}</span>
          </a>
        ))}
      </section>

      <section className="card">
        <h3>我发布的作品</h3>
        {works.length === 0 && <p className="hint">还没有发布过</p>}
        {works.map((item) => (
          <p key={item.id}>
            {item.title} · 赞 {item.like_count}
          </p>
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
    </div>
  );
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  return `${date.getMonth() + 1}月${date.getDate()}日`;
}
