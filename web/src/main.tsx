import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "./App";
import "./styles.css";

// Android 的 WebView 对 safe-area-inset 支持不一致，很多机型直接给 0。
// 在 App 里跑时给一个最小留白兜底，免得顶栏被状态栏压住、底栏被圆角切掉。
const inApp =
  "__TAURI_INTERNALS__" in window || /\bwv\b/.test(navigator.userAgent);

if (inApp) {
  document.documentElement.classList.add("in-app");
}

const container = document.getElementById("root");
if (!container) {
  throw new Error("找不到挂载节点");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>
);
