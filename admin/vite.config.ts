import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  // 后台挂在 /admin 下，资源必须带这个前缀，
  // 否则打包出来的 /assets/xxx.js 会被当成用户端资源去取，页面直接白屏。
  base: "/admin/",
  plugins: [react()],
  server: {
    port: 5174,
    proxy: {
      "/api": "http://127.0.0.1:8080"
    }
  },
  build: { outDir: "dist" }
});
