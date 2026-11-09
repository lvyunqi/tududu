import { useCallback, useEffect, useRef, useState } from "react";

import AuthImage from "../components/AuthImage";
import {
  GalleryIcon,
  GearIcon,
  MenuIcon,
  PlusIcon,
  StarIcon,
  WalletIcon
} from "../components/Icons";
import Lightbox from "../components/Lightbox";
import Toast from "../components/Toast";
import { request } from "../lib/api";
import { readPrefs } from "../lib/prefs";
import type { useAuth } from "../lib/store";
import type {
  Asset,
  Job,
  Message,
  PromptCategory,
  PromptItem,
  PublicModel,
  Session,
  SessionDetail,
  TurnResult
} from "../lib/types";

type Auth = ReturnType<typeof useAuth>;

const FALLBACK_MODEL = "gpt-image-1";

const SUGGESTIONS = [
  "把背景换成海边日落，光线柔和一点",
  "把照片修成胶片质感，颗粒细一点",
  "把这件衣服换成米白色针织衫"
];

export default function ChatPage({ auth, sessionId }: { auth: Auth; sessionId?: string }) {
  const [sid, setSid] = useState<string | null>(sessionId ?? null);
  const [detail, setDetail] = useState<SessionDetail | null>(null);
  const [text, setText] = useState("");
  // 参考图最多 4 张，顺序即发送顺序
  const [assets, setAssets] = useState<Asset[]>([]);
  const [uploading, setUploading] = useState(false);
  const [sending, setSending] = useState(false);
  const [runningJob, setRunningJob] = useState<string | null>(null);
  const [jobPhase, setJobPhase] = useState(0);
  const [startedAt, setStartedAt] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [publishTarget, setPublishTarget] = useState<Message | null>(null);
  const [atBottom, setAtBottom] = useState(true);
  const [models, setModels] = useState<PublicModel[]>([]);
  const [modelsReady, setModelsReady] = useState(false);
  // 记住上次选的模型，别每次进来都被重置成第一个
  const [model, setModel] = useState(
    () => localStorage.getItem("tududu.model") || FALLBACK_MODEL
  );
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [viewing, setViewing] = useState<Message | null>(null);
  const [sessions, setSessions] = useState<Session[]>([]);

  const scrollRef = useRef<HTMLDivElement>(null);
  // 同一个任务只允许一条轮询，切页面回来不会重复起
  const pollingRef = useRef<string | null>(null);
  const aliveRef = useRef(true);

  useEffect(() => {
    aliveRef.current = true;
    return () => {
      aliveRef.current = false;
    };
  }, []);
  const fileRef = useRef<HTMLInputElement>(null);
  // 一次最多四张参考图，再多上游也吃不下
  const MAX_FILES = 4;
  const textRef = useRef<HTMLTextAreaElement>(null);

  const load = useCallback(async (id: string) => {
    const found = await request<SessionDetail>("/api/v1/sessions/" + id).catch(() => null);
    if (found) setDetail(found);
  }, []);

  useEffect(() => {
    if (sid) void load(sid);
  }, [sid, load]);

  // 输入法弹出、页面被压缩时，保持在最新一条上
  useEffect(() => {
    const viewport = window.visualViewport;
    if (!viewport) return;

    const onResize = () => {
      if (atBottom) scrollToBottom();
    };

    viewport.addEventListener("resize", onResize);

    return () => viewport.removeEventListener("resize", onResize);
  }, [atBottom]);

  // 路由上的会话变了（抽屉切换、新对话）时跟上，但不清空正在跑的轮询
  useEffect(() => {
    if (sessionId && sessionId !== sid) {
      setDetail(null);
      setSid(sessionId);
      return;
    }

    // 回到「新对话」时把当前会话清掉，否则会继续显示上一个会话
    if (!sessionId && sid) {
      setDetail(null);
      setSid(null);
    }
  }, [sessionId, sid]);

  // 从相册点「以这张继续改」过来时，把那张图接进输入框
  useEffect(() => {
    const pending = localStorage.getItem("tududu.pendingInput");
    if (pending) {
      localStorage.removeItem("tududu.pendingInput");
      setAssets([{ id: pending } as Asset]);
    }

    // 从广场点「做同款」过来时，把提示词填进输入框
    const prompt = localStorage.getItem("tududu.pendingPrompt");
    if (prompt) {
      localStorage.removeItem("tududu.pendingPrompt");
      setText(prompt);
      requestAnimationFrame(() => textRef.current?.focus());
    }
  }, []);

  // 可选的模型由后台配置决定，一个都没配时退回默认值，让请求照样发出去
  useEffect(() => {
    void request<PublicModel[]>("/api/v1/models", { auth: false })
      .then((list) => {
        setModels(list);

        // 本地没存过选择时才用第一个兜底
        if (list.length > 0 && !localStorage.getItem("tududu.model")) {
          setModel(list[0].code);
        }
      })
      .catch(() => setModels([]))
      .finally(() => setModelsReady(true));
  }, []);

  useEffect(() => {
    void request<Session[]>("/api/v1/sessions?limit=50")
      .then(setSessions)
      .catch(() => setSessions([]));
  }, [sid, drawerOpen]);

  // 切到别的会话时，模型跟着会话走
  useEffect(() => {
    const saved = detail?.session.model;
    if (!sid || !saved) return;

    setModel(saved);
    localStorage.setItem("tududu.model", saved);
  }, [sid, detail?.session.model]);

  // 从别的页面切回正在跑的会话时，接着轮询，不然进度会停在那
  useEffect(() => {
    if (!sid || !detail || runningJob) return;

    const pending = detail.messages.find(
      (item) => item.role !== "user" && item.status === 1 && item.job_id
    );

    if (pending?.job_id) void poll(pending.job_id, sid);
  }, [detail, sid, runningJob]);

  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [detail, runningJob]);

  function onScroll() {
    const el = scrollRef.current;
    if (!el) return;
    setAtBottom(el.scrollHeight - el.scrollTop - el.clientHeight < 80);
  }

  function scrollToBottom() {
    const el = scrollRef.current;
    if (el) el.scrollTo({ top: el.scrollHeight, behavior: "smooth" });
  }

  /// 把已有的图设为下一轮输入，实现「以图改图」。
  async function continueWith(assetId: string) {
    setAssets((prev) =>
      prev.length >= MAX_FILES || prev.some((item) => item.id === assetId)
        ? prev
        : [...prev, { id: assetId } as Asset]
    );
    setError(null);
    textRef.current?.focus();
  }

  /// 从侧边栏或顶栏开一段新对话：清干净当前状态，别把上一段的图和字带过来。
  function startNew() {
    setSid(null);
    setDetail(null);
    setText("");
    setAssets([]);
    setError(null);
    setDrawerOpen(false);
    window.location.hash = "#/";
    requestAnimationFrame(() => textRef.current?.focus());
  }

  /// 删掉一个会话；删的是当前这个就回到新对话。
  async function removeSession(id: string) {
    try {
      await request("/api/v1/sessions/" + id, { method: "DELETE" });
      setSessions((prev) => prev.filter((item) => item.id !== id));

      if (id === sid) {
        setDetail(null);
        setSid(null);
        setDrawerOpen(false);
        window.location.hash = "#/";
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "删除失败");
    }
  }

  async function pickFile(file: File) {
    setUploading(true);
    setError(null);

    try {
      const form = new FormData();
      form.append("file", file);
      const created = await request<Asset>("/api/v1/uploads", { method: "POST", form });
      setAssets((prev) => (prev.length >= MAX_FILES ? prev : [...prev, created]));
    } catch (err) {
      setError(err instanceof Error ? err.message : "上传失败");
    } finally {
      setUploading(false);
    }
  }

  async function send() {
    const content = text.trim();
    if (!content || sending) return;

    setSending(true);
    setError(null);

    try {
      let current = sid;

      if (!current) {
        const created = await request<Session>("/api/v1/sessions", {
          method: "POST",
          body: { model, title: content.slice(0, 20) }
        });

        current = created.id;
        setSid(created.id);
        window.location.hash = "#/session/" + created.id;
      }

      const turn = await request<TurnResult>("/api/v1/sessions/" + current + "/messages", {
        method: "POST",
        body: {
          text: content,
          input_asset_id: assets[0]?.id ?? null,
          input_asset_ids: assets.map((item) => item.id)
        }
      });

      setText("");
      if (!readPrefs().keepInput) setAssets([]);
      if (textRef.current) textRef.current.style.height = "auto";

      await load(current);
      void auth.refresh();
      void poll(turn.job_id, current);
    } catch (err) {
      setError(err instanceof Error ? err.message : "提交失败");
    } finally {
      setSending(false);
    }
  }

  async function poll(jobId: string, id: string) {
    if (pollingRef.current === jobId) return;
    pollingRef.current = jobId;

    setRunningJob(jobId);
    setJobPhase(0);

    try {
      // 开始时间以服务端为准：切页面回来接着算，不会重置
      const first = await request<Job>("/api/v1/jobs/" + jobId).catch(() => null);
      if (first) {
        setJobPhase(first.status);
        setStartedAt(jobStartedAt(first));
      }

      for (let i = 0; i < 240; i += 1) {
        await new Promise((resolve) => setTimeout(resolve, 2500));
        if (!aliveRef.current) return;

        try {
          const latest = await request<Job>("/api/v1/jobs/" + jobId);
          setJobPhase(latest.status);

          if (latest.status === 2 || latest.status === 3) {
            await load(id);
            void auth.refresh();
            return;
          }
        } catch {
          return;
        }
      }
    } finally {
      pollingRef.current = null;
      if (aliveRef.current) setRunningJob(null);
    }
  }

  function grow(event: React.ChangeEvent<HTMLTextAreaElement>) {
    setText(event.target.value);

    const el = event.target;
    el.style.height = "auto";
    el.style.height = Math.min(el.scrollHeight, 140) + "px";
  }

  const messages = detail?.messages ?? [];
  const empty = messages.length === 0 && !runningJob;

  return (
    <div className="chat">
      {/* 下半部分隐约浮着的小方块，让空白背景不那么干 */}
      <div className="chat-particles" aria-hidden="true">
        {PARTICLES.map((item, index) => (
          <span
            key={index}
            style={{
              left: item.left + "%",
              top: item.top + "%",
              width: item.size + "px",
              height: item.size + "px",
              animationDelay: item.delay + "s",
              animationDuration: item.duration + "s"
            }}
          />
        ))}
      </div>

      <div className="chat-tools">
        <button
          className="tool-btn"
          onClick={() => setDrawerOpen(true)}
          aria-label="打开侧边栏"
        >
          <MenuIcon />
        </button>

        <a className="tool-btn" href="#/" onClick={() => startNew()} aria-label="新对话">
          <PlusIcon />
        </a>

        <span className="chat-title">{detail?.session.title ?? "新对话"}</span>

        {/* 广场入口：一颗主火花加一颗小火花 */}
        <a className="plaza-btn" href="#/plaza" aria-label="去广场">
          <StarIcon size={24} />
        </a>
      </div>

      {runningJob && <div className="gen-bar" />}

      <div className="chat-scroll" ref={scrollRef} onScroll={onScroll}>
        {empty && (
          <div className="hero">
            <h1>
              说一句话
              <br />
              剩下的交给图嘟嘟
            </h1>
            <p>上传一张照片，用日常话描述想怎么改。不满意就继续说，它会接着上一版调整。</p>
            <div className="chips">
              {SUGGESTIONS.map((item) => (
                <button key={item} className="chip" onClick={() => setText(item)}>
                  {item}
                </button>
              ))}
            </div>
          </div>
        )}

        {messages.map((message) => (
          <MessageRow
            key={message.id}
            message={message}
            busy={runningJob !== null}
            onOpen={() => setViewing(message)}
            onPublish={() => setPublishTarget(message)}
          />
        ))}

        {runningJob && (
          <div className="msg bot">
            <div className="bubble">
              <span className="who">图嘟嘟</span>
              <Generating phase={jobPhase} startedAt={startedAt} />
            </div>
          </div>
        )}

        {error && (
          <div className="failure inline">
            <span className="failure-icon" aria-hidden="true">
              !
            </span>
            <div className="failure-main">
              <strong>提交失败</strong>
              <p>{error}</p>
            </div>
          </div>
        )}
      </div>

      {!atBottom && (
        <button className="to-bottom" onClick={scrollToBottom} aria-label="回到最新">
          ↓
        </button>
      )}

      <div className="composer">
        {/* 参考图放在输入框上方，单独一条，比塞在框里清楚 */}
        {assets.length > 0 && (
          <div className="attach-bar">
            {assets.map((item, index) => (
              <div key={item.id} className="attach-item">
                <button
                  type="button"
                  className="attach-thumb"
                  onClick={() => setViewing({ output_asset_id: item.id } as Message)}
                  aria-label="查看参考图"
                >
                  <AuthImage assetId={item.id} alt="参考图" variant="thumb" />
                </button>
                <button
                  className="attach-remove"
                  onClick={() => setAssets((prev) => prev.filter((row) => row.id !== item.id))}
                  aria-label="移除参考图"
                >
                  ✕
                </button>
                <span className="attach-label">参考图 {index + 1}</span>
              </div>
            ))}

            {assets.length < MAX_FILES && (
              <button
                className="attach-add"
                onClick={() => fileRef.current?.click()}
                aria-label="再加一张"
              >
                ＋
              </button>
            )}
          </div>
        )}

        <div className="composer-inner">
          {/* 模型没拉回来之前先摆骨架，别让人看见空荡荡一行 */}
          {!modelsReady && (
            <div className="model-row" aria-hidden="true">
              <span className="model-skeleton" />
              <span className="model-skeleton wide" />
              <span className="model-skeleton" />
            </div>
          )}

          {modelsReady && models.length > 0 && (
            <div className="model-row">
              <span className="model-label">模型</span>
              {models.map((item) => (
                <button
                  key={item.code}
                  className={model === item.code ? "chip active" : "chip"}
                  onClick={() => {
                    setModel(item.code);
                    localStorage.setItem("tududu.model", item.code);
                  }}
                  type="button"
                >
                  {item.display_name || item.code}
                </button>
              ))}
            </div>
          )}

          <textarea
            ref={textRef}
            rows={1}
            value={text}
            onChange={grow}
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) void send();
            }}
            placeholder={empty ? "描述你想怎么改，例如：把背景换成海边日落" : "继续说，例如：人物再自然一点"}
          />

          <div className="composer-row">
            <input
              ref={fileRef}
              type="file"
              accept="image/png,image/jpeg,image/webp"
              hidden
              onChange={(event) => {
                const file = event.target.files?.[0];
                if (file) void pickFile(file);
                event.target.value = "";
              }}
            />

            <button
              className="icon-btn"
              onClick={() => fileRef.current?.click()}
              disabled={uploading}
              aria-label="添加图片"
            >
              {uploading ? "…" : "＋"}
            </button>

            <span className="spacer" />

            <button
              className="send-btn"
              onClick={() => void send()}
              disabled={sending || !text.trim()}
              aria-label="发送"
            >
              ↑
            </button>
          </div>
        </div>
      </div>

      {viewing?.output_asset_id && (
        <Lightbox
          assetId={viewing.output_asset_id}
          onClose={() => setViewing(null)}
          onEdit={() => {
            const id = viewing.output_asset_id;
            setViewing(null);
            if (id) void continueWith(id);
          }}
          onPublish={() => {
            const target = viewing;
            setViewing(null);
            setPublishTarget(target);
          }}
        />
      )}

      {publishTarget && (
        <PublishSheet
          message={publishTarget}
          sessionId={sid}
          model={detail?.session.model ?? model}
          defaultPrompt={promptBefore(messages, publishTarget)}
          onClose={() => setPublishTarget(null)}
        />
      )}

      {drawerOpen && (
        <div className="drawer-backdrop" onClick={() => setDrawerOpen(false)}>
          <aside className="drawer" onClick={(event) => event.stopPropagation()}>
            <div className="drawer-profile">
              <div className="drawer-profile-top">
                <div className="avatar">{(auth.user?.nickname ?? "?").slice(0, 1).toUpperCase()}</div>

                <div className="drawer-profile-main">
                  <strong>{auth.user?.nickname}</strong>
                  <span className="hint">
                    {auth.balance} {auth.unitName}
                  </span>
                </div>
              </div>

              <div className="drawer-links">
                <a href="#/me" onClick={() => setDrawerOpen(false)}>
                  <span aria-hidden="true">
                    <GearIcon />
                  </span>
                  设置
                </a>
                <a href="#/recharge" onClick={() => setDrawerOpen(false)}>
                  <span aria-hidden="true">
                    <WalletIcon />
                  </span>
                  钱包
                </a>
                <a href="#/gallery" onClick={() => setDrawerOpen(false)}>
                  <span aria-hidden="true">
                    <GalleryIcon />
                  </span>
                  相册
                </a>
                <a href="#/plaza" onClick={() => setDrawerOpen(false)}>
                  <span aria-hidden="true">
                    <StarIcon size={17} />
                  </span>
                  广场
                </a>
              </div>
            </div>

            <div className="drawer-head">
              <strong>历史对话</strong>
            </div>

            <div className="drawer-list">
              {sessions.length === 0 && <p className="hint">还没有对话</p>}


              {sessions.map((item) => (
                <SwipeRow
                  key={item.id}
                  active={item.id === sid}
                  title={item.title || "未命名对话"}
                  time={shortTime(item.last_active_at)}
                  href={"#/session/" + item.id}
                  onOpen={() => setDrawerOpen(false)}
                  onDelete={() => void removeSession(item.id)}
                />
              ))}
            </div>

            <a className="drawer-new" href="#/" onClick={() => startNew()}>
              <span aria-hidden="true">＋</span>
              新建对话
            </a>
          </aside>
        </div>
      )}
    </div>
  );
}

/// 任务开始时间，优先服务端给的时间戳。
function jobStartedAt(job: Job): number {
  const raw = job.started_at ?? job.created_at;
  const parsed = raw ? Date.parse(raw) : Number.NaN;

  return Number.isFinite(parsed) ? parsed : Date.now();
}

/// 粒子方块的位置和节奏定死，避免每次渲染都跳一下
const PARTICLES = [
  { left: 6, top: 58, size: 10, delay: 0, duration: 11 },
  { left: 18, top: 74, size: 6, delay: 1.6, duration: 13 },
  { left: 27, top: 63, size: 14, delay: 3.1, duration: 15 },
  { left: 38, top: 84, size: 8, delay: 0.7, duration: 12 },
  { left: 47, top: 69, size: 5, delay: 2.4, duration: 14 },
  { left: 56, top: 88, size: 12, delay: 4.2, duration: 16 },
  { left: 66, top: 61, size: 7, delay: 1.1, duration: 12.5 },
  { left: 74, top: 79, size: 16, delay: 2.9, duration: 17 },
  { left: 83, top: 66, size: 6, delay: 0.4, duration: 13.5 },
  { left: 91, top: 86, size: 9, delay: 3.6, duration: 15.5 },
  { left: 12, top: 92, size: 5, delay: 4.8, duration: 14.5 },
  { left: 42, top: 55, size: 4, delay: 5.4, duration: 11.5 },
  { left: 62, top: 95, size: 7, delay: 2.1, duration: 13 },
  { left: 87, top: 57, size: 5, delay: 5.9, duration: 16.5 }
];

/**
 * 一条历史对话。
 *
 * 删除有两个入口，都不摆常驻按钮：
 * - 长按：在条目上方浮出一个小气泡，点「删除」
 * - 左右滑：像推拉门一样把条目推开，露出后面的删除
 */
function SwipeRow({
  active,
  title,
  time,
  href,
  onOpen,
  onDelete
}: {
  active: boolean;
  title: string;
  time: string;
  href: string;
  onOpen: () => void;
  onDelete: () => void;
}) {
  const [offset, setOffset] = useState(0);
  const [dragging, setDragging] = useState(false);

  const startX = useRef<number | null>(null);
  const longPress = useRef<number | null>(null);
  const moved = useRef(false);

  function clearLongPress() {
    if (longPress.current) {
      window.clearTimeout(longPress.current);
      longPress.current = null;
    }
  }

  function onDown(event: React.PointerEvent<HTMLAnchorElement>) {
    startX.current = event.clientX;
    moved.current = false;
    setDragging(true);

    // 长按不是弹气泡，是直接把这一行推开，删除按钮就在同一行里
    longPress.current = window.setTimeout(() => {
      longPress.current = null;
      if (moved.current) return;

      setOffset(-84);
    }, 380);
  }

  function onMove(event: React.PointerEvent<HTMLAnchorElement>) {
    if (startX.current === null) return;

    // 只允许往左拉，往右是死的
    const delta = Math.min(0, event.clientX - startX.current);
    if (Math.abs(delta) > 8) {
      moved.current = true;
      clearLongPress();
    }

    setOffset(Math.max(-84, delta));
  }

  function onUp() {
    clearLongPress();
    setDragging(false);

    if (startX.current === null) return;
    startX.current = null;

    // 拉开得够远才停住，否则弹回去
    if (offset < -42) {
      setOffset(-84);
      return;
    }

    setOffset(0);
  }

  function closeAll() {
    setOffset(0);
  }

  const pulled = offset !== 0;

  return (
    <div
      className={pulled ? "drawer-row open" : "drawer-row"}
    >
      <button
        type="button"
        className="swipe-delete danger"
        tabIndex={pulled ? 0 : -1}
        aria-hidden={!pulled}
        onPointerDown={(event) => event.stopPropagation()}
        onClick={(event) => {
          event.stopPropagation();
          event.preventDefault();
          closeAll();
          onDelete();
        }}
      >
        删除
      </button>

      <a
        className={active ? "drawer-item active" : "drawer-item"}
        href={href}
        draggable={false}
        style={{
          transform: "translateX(" + offset + "px)",
          transition: dragging
            ? "none"
            : "transform 0.34s cubic-bezier(0.34,1.56,0.64,1)"
        }}
        onPointerDown={onDown}
        onPointerMove={onMove}
        onPointerUp={onUp}
        onPointerCancel={onUp}
        onClick={(event) => {
          // 长按、滑动都不该顺带跳转
          if (moved.current || offset !== 0) {
            event.preventDefault();
            if (offset !== 0) closeAll();
            return;
          }

          onOpen();
        }}
        onContextMenu={(event) => event.preventDefault()}
      >
        <span className="drawer-title">{title}</span>
        <span className="drawer-time">{time}</span>
      </a>
    </div>
  );
}

function shortTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";

  const now = Date.now();
  const diff = now - date.getTime();

  if (diff < 60_000) return "刚刚";
  if (diff < 3_600_000) return Math.floor(diff / 60_000) + " 分钟前";
  if (diff < 86_400_000) return Math.floor(diff / 3_600_000) + " 小时前";
  return date.getMonth() + 1 + "月" + date.getDate() + "日";
}

/**
 * 生成状态：一个会形变的标记，环 → 弧 → 对勾。
 *
 * 参考 ReactBits 的 Status Mark 与 Thought Line：
 * 状态不该是文字加等待条，而是一个「活着」的图形，结束时收成一句结论。
 */
function Generating({ phase, startedAt }: { phase: number; startedAt: number }) {
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    if (!startedAt) return;

    const timer = window.setInterval(() => {
      setElapsed((Date.now() - startedAt) / 1000);
    }, 100);

    return () => window.clearInterval(timer);
  }, [startedAt]);

  const label = phase === 0 ? "排队中" : phase === 1 ? "正在生成" : "收尾中";

  return (
    <div className="gen-line">
      <span className="status-mark">
        <span className="glow" />
        <span className="ring" />
        <span className="arc" />
      </span>

      <span className="label">{label}</span>
      <span className="elapsed">{elapsed.toFixed(1)}s</span>
      <span className="fuse" aria-hidden="true" />
    </div>
  );
}

function MessageRow({
  message,
  busy,
  onOpen,
  onPublish
}: {
  message: Message;
  busy: boolean;
  onOpen: () => void;
  onPublish: () => void;
}) {
  const mine = message.role === "user";

  return (
    <div className={mine ? "msg user" : "msg bot"}>
      <div className="bubble">
        {!mine && <span className="who">图嘟嘟</span>}

        {refsOf(message).length > 0 && (
          <div className="bubble-refs">
            {refsOf(message).map((id) => (
              <div key={id} className="bubble-ref">
                <AuthImage assetId={id} alt="参考图" variant="thumb" />
              </div>
            ))}
          </div>
        )}

        {message.content && message.status !== 4 && (
          <p className="prompt">{message.content}</p>
        )}

        {message.output_asset_id && (
          <button className="image-tap" onClick={onOpen} aria-label="查看大图">
            <AuthImage assetId={message.output_asset_id} alt="结果图" variant="thumb" />
          </button>
        )}

        {message.status === 4 && (
          <FailureNotice text={message.content || "这一步没成功，换个说法再试"} />
        )}

        {!mine && message.output_asset_id && (
          <button
            className="chip"
            onClick={onPublish}
            disabled={busy}
            style={{ alignSelf: "flex-start" }}
          >
            发布到广场
          </button>
        )}
      </div>
    </div>
  );
}

/** 失败提示：说清楚为什么、下一步能做什么，别只丢一行红字。 */
function FailureNotice({ text }: { text: string }) {
  const needRecharge = text.includes("余额不足") || text.includes("充值");
  const timeout = text.includes("超时");

  return (
    <div className="failure">
      <span className="failure-icon" aria-hidden="true">
        {timeout ? "⟳" : "!"}
      </span>

      <div className="failure-main">
        <strong>{timeout ? "生成超时" : "这次没生成出来"}</strong>
        <p>{text}</p>

        {needRecharge && (
          <a className="failure-action" href="#/recharge">
            去充值
          </a>
        )}
      </div>
    </div>
  );
}

/// 一条消息带的参考图；老数据只有单图字段，回退到它。
function refsOf(message: Message): string[] {
  if (message.input_asset_ids && message.input_asset_ids.length > 0) {
    return message.input_asset_ids;
  }

  return message.input_asset_id ? [message.input_asset_id] : [];
}

/// 找出这张图对应的那句指令：往前找最近的一条用户消息。
function promptBefore(messages: Message[], target: Message): string {
  const index = messages.findIndex((item) => item.id === target.id);

  for (let i = index - 1; i >= 0; i -= 1) {
    const item = messages[i];
    if (item.role === "user" && item.content.trim()) return item.content.trim();
  }

  return target.content.trim();
}

function PublishSheet({
  message,
  sessionId,
  model,
  defaultPrompt,
  onClose
}: {
  message: Message;
  sessionId: string | null;
  model: string;
  defaultPrompt: string;
  onClose: () => void;
}) {
  const [title, setTitle] = useState((defaultPrompt || "我的作品").slice(0, 24));
  // 提示词自动读对话里的那句指令，可以改
  const [promptText, setPromptText] = useState(defaultPrompt);
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [categoryName, setCategoryName] = useState("");
  const [categories, setCategories] = useState<PromptCategory[]>([]);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  useEffect(() => {
    void request<PromptCategory[]>("/api/v1/prompt-categories", { auth: false })
      .then(setCategories)
      .catch(() => setCategories([]));
  }, []);

  async function submit() {
    setBusy(true);
    setNote(null);

    try {
      const created = await request<PromptItem>("/api/v1/prompts", {
        method: "POST",
        body: {
          title: title.trim(),
          prompt_text: promptText.trim(),
          model,
          category_id: categoryName.trim() ? null : categoryId,
          category_name: categoryName.trim() || null,
          cover_asset_id: message.output_asset_id,
          source_session_id: sessionId
        }
      });

      setNote(created.status === 1 ? "已提交，等待审核" : "已发布");
      setTimeout(onClose, 900);
    } catch (err) {
      setNote(err instanceof Error ? err.message : "发布失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="sheet-backdrop" onClick={onClose}>
      <div className="sheet" onClick={(event) => event.stopPropagation()}>
        <h3>发布到广场</h3>
        <p className="hint">别人可以用你这句描述做出同款效果。</p>

        <input
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          placeholder="给作品起个名字"
        />

        <textarea
          rows={3}
          value={promptText}
          onChange={(event) => setPromptText(event.target.value)}
          placeholder="提示词：描述这张图是怎么做出来的"
        />

        <input
          value={categoryName}
          onChange={(event) => setCategoryName(event.target.value)}
          placeholder="自定义分类（填了就不用选下面的）"
        />

        <div className="chips">
          {categories.map((item) => (
            <button
              key={item.id}
              className={categoryId === item.id && !categoryName.trim() ? "chip active" : "chip"}
              onClick={() => {
                setCategoryId(item.id);
                setCategoryName("");
              }}
            >
              {item.name}
            </button>
          ))}
        </div>

        {note && <Toast text={note} onDone={() => setNote(null)} />}

        <div className="composer-row">
          <button className="ghost" onClick={onClose}>
            取消
          </button>
          <span className="spacer" />
          <button
            onClick={() => void submit()}
            disabled={busy || !title.trim() || !promptText.trim()}
          >
            {busy ? "提交中…" : "发布"}
          </button>
        </div>
      </div>
    </div>
  );
}
