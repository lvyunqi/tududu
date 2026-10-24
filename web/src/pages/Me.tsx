import { useEffect, useState } from "react";

import { request } from "../lib/api";
import type { useAuth } from "../lib/store";
import type { PromptItem } from "../lib/types";

type Auth = ReturnType<typeof useAuth>;

export default function MePage({ auth, onRecharge }: { auth: Auth; onRecharge: () => void }) {
  const [works, setWorks] = useState<PromptItem[]>([]);

  useEffect(() => {
    void request<PromptItem[]>("/api/v1/my/prompts?limit=20")
      .then(setWorks)
      .catch(() => setWorks([]));
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
        <button onClick={onRecharge}>去充值</button>
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

      <button className="danger" onClick={() => void auth.logout()}>
        退出登录
      </button>
    </div>
  );
}
