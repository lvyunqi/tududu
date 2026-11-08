import { useEffect, useState } from "react";

import Toast from "../components/Toast";
import { BASE, request, requestBlob } from "../lib/api";
import type { useAuth } from "../lib/store";
import { readPrefs, writePrefs } from "../lib/prefs";
import type { PromptItem } from "../lib/types";

type Auth = ReturnType<typeof useAuth>;

interface Quota {
  used_bytes: number;
  limit_bytes: number;
}

export default function MePage({ auth }: { auth: Auth }) {
  const [works, setWorks] = useState<PromptItem[]>([]);
  const [quota, setQuota] = useState<Quota | null>(null);
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

    void request<Quota>("/api/v1/storage/quota")
      .then(setQuota)
      .catch(() => setQuota(null));
  }, []);

  return (
    <div className="page">
      <a className="back-home" href="#/" aria-label="回到对话">
        ←
      </a>

      <ProfileCard auth={auth} onToast={setToast} />

      <section className="card">
        <div className="card-head">
          <h3>余额</h3>
          <span className="tag">{auth.unitName}</span>
        </div>

        <p className="balance-line">
          <strong>{auth.balance}</strong>
          <span>{auth.unitName}</span>
        </p>

        <a className="button" href="#/recharge">
          去充值
        </a>
      </section>

      {quota && (
        <section className="card">
          <div className="card-head">
            <h3>存储占用</h3>
            <span className="tag">{percentOf(quota)}%</span>
          </div>

          <div className="meter" aria-hidden="true">
            <span style={{ width: percentOf(quota) + "%" }} />
          </div>

          <p className="hint">
            {formatBytes(quota.used_bytes)} / {formatBytes(quota.limit_bytes)}
          </p>
        </section>
      )}

      <PreferenceCard onToast={setToast} />

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

      <SecurityCard onToast={setToast} auth={auth} />

      <section className="card">
        <h3>关于</h3>
        <p className="hint">接口地址：{BASE || "当前站点同源"}</p>
        <p className="hint">注册时间：{formatDay(auth.user?.created_at)}</p>
        <p className="hint">版本：{import.meta.env.MODE}</p>
      </section>

      {toast && <Toast text={toast} onDone={() => setToast(null)} />}
    </div>
  );
}

/* ---------------- 个人资料 ---------------- */

function ProfileCard({
  auth,
  onToast
}: {
  auth: Auth;
  onToast: (text: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(auth.user?.nickname ?? "");
  const [busy, setBusy] = useState(false);

  async function save() {
    const next = name.trim();
    if (!next) return;

    setBusy(true);

    try {
      await request("/api/v1/me", { method: "PATCH", body: { nickname: next } });
      await auth.refresh();
      setEditing(false);
      onToast("昵称已更新");
    } catch (err) {
      onToast(err instanceof Error ? err.message : "保存失败");
    } finally {
      setBusy(false);
    }
  }

  const initial = (auth.user?.nickname ?? "?").slice(0, 1).toUpperCase();

  return (
    <section className="card profile-card">
      <div className="avatar">{initial}</div>

      <div className="profile-main">
        {editing ? (
          <div className="profile-edit">
            <input
              value={name}
              maxLength={24}
              onChange={(event) => setName(event.target.value)}
              placeholder="起个名字"
              autoFocus
            />
            <button className="mini" onClick={() => void save()} disabled={busy || !name.trim()}>
              {busy ? "保存中" : "保存"}
            </button>
            <button className="mini" onClick={() => setEditing(false)} disabled={busy}>
              取消
            </button>
          </div>
        ) : (
          <>
            <h2>{auth.user?.nickname}</h2>
            <span className="hint">编号 {auth.user?.short_code}</span>
          </>
        )}
      </div>

      {!editing && (
        <button className="mini" onClick={() => setEditing(true)}>
          改名
        </button>
      )}
    </section>
  );
}

/* ---------------- 偏好设置 ---------------- */

function PreferenceCard({ onToast }: { onToast: (text: string) => void }) {
  const [prefs, setPrefs] = useState(readPrefs);

  function update(patch: Partial<typeof prefs>) {
    const next = { ...prefs, ...patch };
    setPrefs(next);
    writePrefs(next);
    onToast("已保存");
  }

  return (
    <section className="card">
      <div className="card-head">
        <h3>偏好</h3>
        <span className="tag">存在本机</span>
      </div>

      <label className="switch-row">
        <span>
          减少动效
          <em>关掉弹入、粒子这些动画，省电也更稳</em>
        </span>
        <input
          type="checkbox"
          checked={prefs.reduceMotion}
          onChange={(event) => update({ reduceMotion: event.target.checked })}
        />
      </label>

      <label className="switch-row">
        <span>
          发送后保留参考图
          <em>开着方便连着改几轮</em>
        </span>
        <input
          type="checkbox"
          checked={prefs.keepInput}
          onChange={(event) => update({ keepInput: event.target.checked })}
        />
      </label>
    </section>
  );
}

/* ---------------- 账号安全 ---------------- */

function SecurityCard({ auth, onToast }: { auth: Auth; onToast: (text: string) => void }) {
  const [open, setOpen] = useState(false);
  const [oldPassword, setOld] = useState("");
  const [newPassword, setNew] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit() {
    setBusy(true);

    try {
      await request("/api/v1/me/password", {
        method: "POST",
        body: { old_password: oldPassword, new_password: newPassword }
      });

      onToast("密码已改，请重新登录");
      setOpen(false);
      setOld("");
      setNew("");
      window.setTimeout(() => void auth.logout(), 900);
    } catch (err) {
      onToast(err instanceof Error ? err.message : "改密码失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="card">
      <div className="card-head">
        <h3>账号安全</h3>
        <button className="mini" onClick={() => setOpen((prev) => !prev)}>
          {open ? "收起" : "改密码"}
        </button>
      </div>

      {open ? (
        <div className="password-form">
          <input
            type="password"
            value={oldPassword}
            onChange={(event) => setOld(event.target.value)}
            placeholder="原密码"
            autoComplete="current-password"
          />
          <input
            type="password"
            value={newPassword}
            onChange={(event) => setNew(event.target.value)}
            placeholder="新密码，至少 8 位"
            autoComplete="new-password"
          />
          <button
            onClick={() => void submit()}
            disabled={busy || !oldPassword || newPassword.length < 8}
          >
            {busy ? "提交中…" : "确认修改"}
          </button>
          <p className="hint">改完其它设备会被登出。</p>
        </div>
      ) : (
        <button className="danger" onClick={() => void auth.logout()}>
          退出登录
        </button>
      )}
    </section>
  );
}

/* ---------------- 作品行 ---------------- */

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

  async function call(path: string, method: string, done: string) {
    setStage("busy");

    try {
      await request(path, { method });
      onToast(done);
      onChanged();
      setStage("idle");
    } catch (err) {
      onToast(err instanceof Error ? err.message : "操作失败");
      setStage("idle");
    }
  }

  function hide() {
    void call("/api/v1/prompts/" + item.short_code, "DELETE", "已从广场撤下");
  }

  function republish() {
    void call("/api/v1/prompts/" + item.short_code + "/republish", "POST", "已重新上架");
  }

  async function purge() {
    setStage("busy");

    try {
      await request("/api/v1/prompts/" + item.short_code + "/permanent", {
        method: "DELETE"
      });

      setStage("leaving");
      window.setTimeout(() => onRemoved(item.id), 260);
      onToast("作品已删除");
    } catch (err) {
      onToast(err instanceof Error ? err.message : "删除失败");
      setStage("idle");
    }
  }

  const busy = stage === "busy";

  return (
    <div className={stage === "leaving" ? "row work-row leaving" : "row work-row"}>
      {item.cover_asset_id && (
        <div className="work-cover">
          <WorkThumb id={item.cover_asset_id} />
        </div>
      )}

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
              <button className="mini" onClick={hide} disabled={busy}>
                {busy ? "处理中" : "隐藏"}
              </button>
            )}
            {item.status === 3 && (
              <button className="mini" onClick={republish} disabled={busy}>
                {busy ? "处理中" : "重新上架"}
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

/** 作品封面要带令牌才取得到。 */
function WorkThumb({ id }: { id: string }) {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    let revoked = false;
    let objectUrl: string | null = null;

    void requestBlob("/api/v1/assets/" + id + "/file?size=thumb")
      .then((blob) => {
        objectUrl = URL.createObjectURL(blob);
        if (!revoked) setUrl(objectUrl);
      })
      .catch(() => setUrl(null));

    return () => {
      revoked = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [id]);

  if (!url) return <div className="work-cover-ph" />;
  return <img src={url} alt="" loading="lazy" />;
}

function percentOf(quota: Quota): number {
  if (quota.limit_bytes <= 0) return 0;
  return Math.min(100, Math.round((quota.used_bytes / quota.limit_bytes) * 100));
}

function formatBytes(value: number): string {
  if (value < 1024) return value + " B";
  if (value < 1024 * 1024) return (value / 1024).toFixed(1) + " KB";
  if (value < 1024 * 1024 * 1024) return (value / 1024 / 1024).toFixed(1) + " MB";
  return (value / 1024 / 1024 / 1024).toFixed(2) + " GB";
}

function formatDay(value?: string): string {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";

  return date.getFullYear() + " 年 " + (date.getMonth() + 1) + " 月 " + date.getDate() + " 日";
}
