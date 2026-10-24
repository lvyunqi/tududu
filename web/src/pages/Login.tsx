import { useState } from "react";

import type { useAuth } from "../lib/store";

type Auth = ReturnType<typeof useAuth>;

export default function LoginPage({ auth }: { auth: Auth }) {
  const [mode, setMode] = useState<"login" | "register">("login");
  const [phone, setPhone] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);

    try {
      if (mode === "login") {
        await auth.login(phone, password);
      } else {
        await auth.register(phone, password);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "操作失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="login">
      <h1>图嘟嘟</h1>
      <p className="hint">用一句话改图</p>

      <form onSubmit={submit} className="card">
        <label>
          手机号
          <input
            value={phone}
            onChange={(event) => setPhone(event.target.value)}
            inputMode="numeric"
            autoComplete="tel"
            placeholder="11 位手机号"
          />
        </label>

        <label>
          密码
          <input
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            autoComplete={mode === "login" ? "current-password" : "new-password"}
            placeholder="至少 8 位，含字母和数字"
          />
        </label>

        {error && <p className="error">{error}</p>}

        <button type="submit" disabled={busy}>
          {busy ? "处理中…" : mode === "login" ? "登录" : "注册"}
        </button>

        <button
          type="button"
          className="link"
          onClick={() => setMode(mode === "login" ? "register" : "login")}
        >
          {mode === "login" ? "还没有账号，去注册" : "已有账号，去登录"}
        </button>
      </form>
    </div>
  );
}
