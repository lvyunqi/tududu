import { useEffect, useState } from "react";

import { useAuth } from "./lib/store";
import CreatePage from "./pages/Create";
import GalleryPage from "./pages/Gallery";
import LoginPage from "./pages/Login";
import MePage from "./pages/Me";
import PlazaPage from "./pages/Plaza";
import RechargePage from "./pages/Recharge";
import SessionDetailPage from "./pages/SessionDetail";

const TABS = [
  { key: "create", label: "创作" },
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
    route.kind === "tab" ? route.tab : route.kind === "recharge" ? "me" : "me";

  return (
    <div className="shell">
      <header className="topbar">
        <div className="brand">图嘟嘟</div>
        <div className="balance">
          {auth.balance} {auth.unitName}
        </div>
      </header>

      <main className="content">
        {route.kind === "tab" && route.tab === "create" && <CreatePage auth={auth} />}
        {route.kind === "tab" && route.tab === "plaza" && <PlazaPage />}
        {route.kind === "tab" && route.tab === "gallery" && <GalleryPage />}
        {route.kind === "tab" && route.tab === "me" && <MePage auth={auth} />}
        {route.kind === "recharge" && <RechargePage auth={auth} />}
        {route.kind === "session" && (
          <SessionDetailPage auth={auth} sessionId={route.id} />
        )}
      </main>

      <nav className="tabbar">
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
