import { useState } from "react";

import { adminRequest, getAdminToken, setAdminToken } from "./lib/api";
import BillingPage from "./pages/Billing";
import JobsPage from "./pages/Jobs";
import ModelsPage from "./pages/Models";
import OrdersPage from "./pages/Orders";
import OverviewPage from "./pages/Overview";
import PayChannelsPage from "./pages/PayChannels";
import PromptsPage from "./pages/Prompts";
import ProvidersPage from "./pages/Providers";
import SettingsPage from "./pages/Settings";
import UsersPage from "./pages/Users";

const SECTIONS = [
  { key: "overview", label: "看板" },
  { key: "users", label: "用户" },
  { key: "orders", label: "充值" },
  { key: "jobs", label: "任务" },
  { key: "prompts", label: "广场" },
  { key: "providers", label: "渠道" },
  { key: "models", label: "模型" },
  { key: "billing", label: "计费" },
  { key: "pay", label: "支付" },
  { key: "settings", label: "配置" }
] as const;

type SectionKey = (typeof SECTIONS)[number]["key"];

export default function App() {
  const [token, setToken] = useState(getAdminToken());
  const [section, setSection] = useState<SectionKey>("overview");

  if (!token) {
    return <LoginPage onDone={setToken} />;
  }

  return (
    <div className="layout">
      <aside className="sidebar">
        <div className="brand">图嘟嘟</div>
        {SECTIONS.map((item) => (
          <button
            key={item.key}
            className={item.key === section ? "nav active" : "nav"}
            onClick={() => setSection(item.key)}
          >
            {item.label}
          </button>
        ))}
        <button
          className="nav logout"
          onClick={() => {
            setAdminToken("");
            setToken("");
          }}
        >
          退出
        </button>
      </aside>

      <main className="main">
        {section === "overview" && <OverviewPage />}
        {section === "users" && <UsersPage />}
        {section === "orders" && <OrdersPage />}
        {section === "jobs" && <JobsPage />}
        {section === "prompts" && <PromptsPage />}
        {section === "providers" && <ProvidersPage />}
        {section === "models" && <ModelsPage />}
        {section === "billing" && <BillingPage />}
        {section === "pay" && <PayChannelsPage />}
        {section === "settings" && <SettingsPage />}
      </main>
    </div>
  );
}

function LoginPage({ onDone }: { onDone: (token: string) => void }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);

    try {
      // 登录接口本身不需要令牌，先清空再发
      setAdminToken("");
      const result = await adminRequest<{ access_token: string }>("/api/v1/admin/auth/login", {
        method: "POST",
        body: { username, password }
      });

      setAdminToken(result.access_token);
      onDone(result.access_token);
    } catch (err) {
      setError(err instanceof Error ? err.message : "登录失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="login" onSubmit={submit}>
      <h1>图嘟嘟 管理后台</h1>
      <input
        placeholder="账号"
        value={username}
        onChange={(event) => setUsername(event.target.value)}
        autoComplete="username"
      />
      <input
        type="password"
        placeholder="密码"
        value={password}
        onChange={(event) => setPassword(event.target.value)}
        autoComplete="current-password"
      />
      {error && <p className="error">{error}</p>}
      <button type="submit" disabled={busy || !username || !password}>
        {busy ? "登录中…" : "登录"}
      </button>
    </form>
  );
}
