import { useEffect, useState } from "react";

import { useAuth } from "./lib/store";
import CreatePage from "./pages/Create";
import GalleryPage from "./pages/Gallery";
import LoginPage from "./pages/Login";
import MePage from "./pages/Me";
import PlazaPage from "./pages/Plaza";
import RechargePage from "./pages/Recharge";

const TABS = [
  { key: "create", label: "创作" },
  { key: "plaza", label: "广场" },
  { key: "gallery", label: "相册" },
  { key: "me", label: "我的" }
] as const;

type TabKey = (typeof TABS)[number]["key"];
type Route = TabKey | "recharge";

function currentRoute(): Route {
  const hash = window.location.hash.replace("#/", "");
  if (hash === "recharge") return "recharge";

  const found = TABS.find((tab) => tab.key === hash);
  return found ? found.key : "create";
}

export default function App() {
  const auth = useAuth();
  const [route, setRoute] = useState<Route>(currentRoute);

  useEffect(() => {
    const onHash = () => setRoute(currentRoute());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  function go(next: Route) {
    window.location.hash = `#/${next}`;
  }

  if (auth.loading) {
    return <div className="boot">正在加载…</div>;
  }

  if (!auth.user) {
    return <LoginPage auth={auth} />;
  }

  const activeTab: TabKey = route === "recharge" ? "me" : route;

  return (
    <div className="shell">
      <header className="topbar">
        <div className="brand">图嘟嘟</div>
        <div className="balance">
          {auth.balance} {auth.unitName}
        </div>
      </header>

      <main className="content">
        {route === "create" && <CreatePage auth={auth} />}
        {route === "plaza" && <PlazaPage />}
        {route === "gallery" && <GalleryPage />}
        {route === "me" && <MePage auth={auth} onRecharge={() => go("recharge")} />}
        {route === "recharge" && (
          <RechargePage auth={auth} onBack={() => go("me")} />
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
