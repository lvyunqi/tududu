import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { useAuth } from "./lib/store";
import ChatPage from "./pages/Chat";
import GalleryPage from "./pages/Gallery";
import LoginPage from "./pages/Login";
import MePage from "./pages/Me";
import PlazaPage from "./pages/Plaza";
import RechargePage from "./pages/Recharge";

const TABS = [
  { key: "create", label: "对话" },
  { key: "plaza", label: "广场" },
  { key: "gallery", label: "相册" },
  { key: "me", label: "我的" }
] as const;

type TabKey = (typeof TABS)[number]["key"];

type Route =
  | { kind: "tab"; tab: TabKey }
  | { kind: "recharge" }
  | { kind: "session"; id: string };

function parseRoute(): Route {
  const hash = window.location.hash.replace("#/", "");

  if (hash === "recharge") return { kind: "recharge" };
  if (hash.startsWith("session/")) {
    return { kind: "session", id: hash.slice("session/".length) };
  }

  const found = TABS.find((tab) => tab.key === hash);
  return { kind: "tab", tab: found ? found.key : "create" };
}

export default function App() {
  const auth = useAuth();
  const [route, setRoute] = useState<Route>(parseRoute);

  useEffect(() => {
    const onHash = () => setRoute(parseRoute());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  if (auth.loading) {
    return <div className="boot">正在加载…</div>;
  }

  if (!auth.user) {
    return <LoginPage auth={auth} />;
  }

  const activeTab: TabKey =
    route.kind === "tab" ? route.tab : route.kind === "recharge" ? "me" : "create";

  // 对话页在「新建」和「某个会话」之间切换时必须占同一个节点，
  // 否则 React 会把它卸载重建，刚发出的消息和生成动画会丢。
  const inChat = (route.kind === "tab" && route.tab === "create") || route.kind === "session";
  const chatSessionId = route.kind === "session" ? route.id : undefined;

  // 底部导航的滑块跟着选中项滑过去，而不是整块换色
  const navRef = useRef<HTMLElement>(null);
  const [pill, setPill] = useState({ left: 0, width: 0 });

  useLayoutEffect(() => {
    const nav = navRef.current;
    const active = nav?.querySelector<HTMLElement>(".tab.active");
    if (!active) return;

    setPill({ left: active.offsetLeft, width: active.offsetWidth });
  }, [activeTab, auth.user]);

  return (
    <div className="shell">
      <header className="topbar">
        <div className="brand">图嘟嘟</div>
        <div className="balance">
          {auth.balance} {auth.unitName}
        </div>
      </header>

      <main className="content">
        {inChat && <ChatPage auth={auth} sessionId={chatSessionId} />}
        {route.kind === "tab" && route.tab === "plaza" && <PlazaPage />}
        {route.kind === "tab" && route.tab === "gallery" && <GalleryPage />}
        {route.kind === "tab" && route.tab === "me" && <MePage auth={auth} />}
        {route.kind === "recharge" && <RechargePage auth={auth} />}
      </main>

      <nav className="tabbar" ref={navRef}>
        <span
          className="tab-indicator"
          style={{ left: pill.left, width: pill.width }}
          aria-hidden="true"
        />

        {TABS.map((item) => (
          <a
            key={item.key}
            className={item.key === activeTab ? "tab active" : "tab"}
            href={`#/${item.key}`}
          >
            {item.label}
          </a>
        ))}
      </nav>
    </div>
  );
}
