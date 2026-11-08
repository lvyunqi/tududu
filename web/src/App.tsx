import { useEffect, useState } from "react";

import { useAuth } from "./lib/store";
import ChatPage from "./pages/Chat";
import GalleryPage from "./pages/Gallery";
import LoginPage from "./pages/Login";
import MePage from "./pages/Me";
import PlazaPage from "./pages/Plaza";
import RechargePage from "./pages/Recharge";

type Route =
  | { kind: "chat" }
  | { kind: "plaza" }
  | { kind: "gallery" }
  | { kind: "me" }
  | { kind: "recharge" }
  | { kind: "session"; id: string };

function parseRoute(): Route {
  const hash = window.location.hash.replace("#/", "");

  if (hash === "plaza") return { kind: "plaza" };
  if (hash === "gallery") return { kind: "gallery" };
  if (hash === "me") return { kind: "me" };
  if (hash === "recharge") return { kind: "recharge" };
  if (hash.startsWith("session/")) {
    return { kind: "session", id: hash.slice("session/".length) };
  }

  return { kind: "chat" };
}

/**
 * 让页面高度跟着「可见区域」走，而不是跟着窗口走。
 *
 * 手机弹出输入法时 WebView 会把整个窗口顶上去，布局跟着抖。
 * 用 visualViewport 拿到真实可见高度，写进 --app-height，
 * 页面自己缩，输入框贴着键盘，不会整页跳。
 */
function useViewportHeight() {
  useEffect(() => {
    const viewport = window.visualViewport;
    const root = document.documentElement;

    function apply() {
      const height = viewport?.height ?? window.innerHeight;
      const keyboard = Math.max(0, window.innerHeight - height);

      root.style.setProperty("--app-height", height + "px");
      root.style.setProperty("--keyboard-inset", keyboard + "px");
      root.classList.toggle("keyboard-open", keyboard > 120);
    }

    apply();
    viewport?.addEventListener("resize", apply);
    viewport?.addEventListener("scroll", apply);
    window.addEventListener("resize", apply);

    return () => {
      viewport?.removeEventListener("resize", apply);
      viewport?.removeEventListener("scroll", apply);
      window.removeEventListener("resize", apply);
    };
  }, []);
}

export default function App() {
  // 所有 hook 必须在任何 return 之前
  const auth = useAuth();
  const [route, setRoute] = useState<Route>(parseRoute);

  useViewportHeight();

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

  // 对话页在「新建」和「某个会话」之间必须占同一个节点，否则会被卸载重建
  const inChat = route.kind === "chat" || route.kind === "session";
  const chatSessionId = route.kind === "session" ? route.id : undefined;

  // 换页面时整块淡入上浮；对话页内部切换会话不算换页，所以 key 固定是 chat
  const pageKey = inChat ? "chat" : route.kind;

  return (
    <div className="shell">
      <main className="content">
        <div className="page-switch" key={pageKey}>
          {inChat && <ChatPage auth={auth} sessionId={chatSessionId} />}
          {route.kind === "plaza" && <PlazaPage />}
          {route.kind === "gallery" && <GalleryPage />}
          {route.kind === "me" && <MePage auth={auth} />}
          {route.kind === "recharge" && <RechargePage auth={auth} />}
        </div>
      </main>
    </div>
  );
}
