import { useEffect, useState } from "react";

import { useAuth } from "./lib/store";
import CreatePage from "./pages/Create";
import GalleryPage from "./pages/Gallery";
import LoginPage from "./pages/Login";
import MePage from "./pages/Me";
import PlazaPage from "./pages/Plaza";

const TABS = [
  { key: "create", label: "创作" },
  { key: "plaza", label: "广场" },
  { key: "gallery", label: "相册" },
  { key: "me", label: "我的" }
] as const;

type TabKey = (typeof TABS)[number]["key"];

function currentTab(): TabKey {
  const hash = window.location.hash.replace("#/", "");
  const found = TABS.find((tab) => tab.key === hash);
  return found ? found.key : "create";
}

export default function App() {
  const auth = useAuth();
  const [tab, setTab] = useState<TabKey>(currentTab);

  useEffect(() => {
    const onHash = () => setTab(currentTab());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  if (auth.loading) {
    return <div className="boot">正在加载…</div>;
  }

  if (!auth.user) {
    return <LoginPage auth={auth} />;
  }

  return (
    <div className="shell">
      <header className="topbar">
        <div className="brand">图嘟嘟</div>
        <div className="balance">
          {auth.balance} {auth.unitName}
        </div>
      </header>

      <main className="content">
        {tab === "create" && <CreatePage auth={auth} />}
        {tab === "plaza" && <PlazaPage />}
        {tab === "gallery" && <GalleryPage />}
        {tab === "me" && <MePage auth={auth} />}
      </main>

      <nav className="tabbar">
        {TABS.map((item) => (
          <a
            key={item.key}
            className={item.key === tab ? "tab active" : "tab"}
            href={`#/${item.key}`}
          >
            {item.label}
          </a>
        ))}
      </nav>
    </div>
  );
}
