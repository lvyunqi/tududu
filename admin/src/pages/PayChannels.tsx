import { useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { PayChannel, SettingItem } from "../lib/types";

const PAY_TYPES = [
  { value: "alipay", label: "支付宝" },
  { value: "wxpay", label: "微信支付" }
];

export default function PayChannelsPage() {
  const channels = useLoad(() => adminRequest<PayChannel[]>("/api/v1/admin/pay-channels"));
  const settings = useLoad(() => adminRequest<SettingItem[]>("/api/v1/admin/settings"));

  const [name, setName] = useState("");
  const [gateway, setGateway] = useState("");
  const [pid, setPid] = useState("");
  const [secret, setSecret] = useState("");
  const [payType, setPayType] = useState("alipay");
  const [note, setNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const baseUrl = (settings.data ?? []).find((item) => item.key === "public_base_url")?.value ?? "";
  const notifyUrl = baseUrl ? baseUrl + "/pay/epay/notify" : "请先在「配置」里填对外访问地址";

  async function create() {
    setBusy(true);
    setNote(null);

    try {
      await adminRequest("/api/v1/admin/pay-channels", {
        method: "POST",
        body: {
          name: name.trim(),
          gateway_url: gateway.trim(),
          pid: pid.trim(),
          key: secret.trim(),
          pay_type: payType
        }
      });

      setName("");
      setGateway("");
      setPid("");
      setSecret("");
      setNote("已添加");
      await channels.reload();
    } catch (err) {
      setNote(err instanceof Error ? err.message : "添加失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section>
      <header className="page-head">
        <h2>支付通道</h2>
        <button onClick={() => void channels.reload()}>刷新</button>
      </header>

      <p className="hint">
        易支付后台里的异步通知地址填这个：<code>{notifyUrl}</code>
      </p>

      <div className="panel">
        <h3>新增通道</h3>
        <div className="form-grid">
          <label>
            名称
            <input value={name} onChange={(e) => setName(e.target.value)} placeholder="例如：易支付支付宝" />
          </label>
          <label>
            网关地址
            <input
              value={gateway}
              onChange={(e) => setGateway(e.target.value)}
              placeholder="https://pay.example.com"
            />
          </label>
          <label>
            商户号（PID）
            <input value={pid} onChange={(e) => setPid(e.target.value)} placeholder="1000" />
          </label>
          <label>
            商户密钥
            <input
              type="password"
              value={secret}
              onChange={(e) => setSecret(e.target.value)}
              placeholder="在易支付后台获取"
            />
          </label>
          <label>
            支付方式
            <select value={payType} onChange={(e) => setPayType(e.target.value)}>
              {PAY_TYPES.map((item) => (
                <option key={item.value} value={item.value}>
                  {item.label}
                </option>
              ))}
            </select>
          </label>
        </div>

        {note && <p className="hint">{note}</p>}

        <button onClick={() => void create()} disabled={busy || !name || !gateway || !pid || !secret}>
          {busy ? "提交中…" : "添加"}
        </button>
      </div>

      <div className="panel">
        <h3>已有通道</h3>
        <table>
          <thead>
            <tr>
              <th>名称</th>
              <th>网关</th>
              <th>商户号</th>
              <th>方式</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            {(channels.data ?? []).map((item) => (
              <tr key={item.id}>
                <td>{item.name}</td>
                <td className="mono">{item.gateway_url}</td>
                <td className="mono">{item.pid}</td>
                <td>{item.pay_type}</td>
                <td>{item.enabled ? "启用" : "停用"}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {(channels.data ?? []).length === 0 && <p className="hint">还没有通道，不配就没法充值</p>}
      </div>
    </section>
  );
}
