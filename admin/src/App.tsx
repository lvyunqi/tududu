import { useState } from "react";

import { getAdminToken, setAdminToken } from "./lib/api";
import BillingPage from "./pages/Billing";
import JobsPage from "./pages/Jobs";
import OrdersPage from "./pages/Orders";
import OverviewPage from "./pages/Overview";
import PromptsPage from "./pages/Prompts";
import ProvidersPage from "./pages/Providers";
import UsersPage from "./pages/Users";

const SECTIONS = [
  { key: "overview", label: "看板" },
  { key: "users", label: "用户" },
  { key: "orders", label: "充值" },
  { key: "jobs", label: "任务" },
  { key: "prompts", label: "广场" },
  { key: "providers", label: "渠道" },
  { key: "billing", label: "计费" }
] as const;

type SectionKey = (typeof SECTIONS)[number]["key"];

export default function App() {
  const [token, setToken] = useState(getAdminToken());
  const [draft, setDraft] = useState("");
  const [section, setSection] = useState<SectionKey>("overview");

  if (!token) {
    return (
      <div className="login">
        <h1>图嘟嘟 管理后台</h1>
        <p className="hint">填入服务端配置的管理令牌</p>
        <input
          type="password"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="TUDUDU_ADMIN_TOKEN"
        />
        <button
          onClick={() => {
            setAdminToken(draft.trim());
            setToken(draft.trim());
          }}
          disabled={!draft.trim()}
        >
          进入
        </button>
      </div>
    );
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
        {section === "billing" && <BillingPage />}
      </main>
    </div>
  );
}
