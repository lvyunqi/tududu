import { useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { AdminOrder, RechargePlan } from "../lib/types";
import { ORDER_STATUS } from "../lib/types";

export default function OrdersPage() {
  const orders = useLoad(() => adminRequest<AdminOrder[]>("/api/v1/admin/pay-orders?limit=50"));

  if (orders.loading) return <p className="hint">加载中…</p>;
  if (orders.error) return <p className="error">{orders.error}</p>;

  return (
    <section>
      <h2>充值</h2>

      <PlanPanel />

      <h3>充值订单</h3>
      <table>
        <thead>
          <tr>
            <th>订单号</th>
            <th>金额</th>
            <th>到账</th>
            <th>状态</th>
            <th>创建时间</th>
            <th>支付时间</th>
          </tr>
        </thead>
        <tbody>
          {(orders.data ?? []).map((order) => (
            <tr key={order.order_no}>
              <td className="mono">{order.order_no}</td>
              <td className="mono">¥{(order.amount_fen / 100).toFixed(2)}</td>
              <td className="mono">
                {order.credit}
                {order.bonus > 0 ? " + " + order.bonus : ""}
              </td>
              <td>{ORDER_STATUS[order.status] ?? order.status}</td>
              <td className="mono">{formatTime(order.created_at)}</td>
              <td className="mono">{order.paid_at ? formatTime(order.paid_at) : "—"}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {(orders.data ?? []).length === 0 && <p className="hint">还没有订单</p>}
    </section>
  );
}

/** 套餐是充值的入口，没有套餐用户就看不到可买的档位。 */
function PlanPanel() {
  const plans = useLoad(() => adminRequest<RechargePlan[]>("/api/v1/admin/recharge-plans"));

  const [name, setName] = useState("");
  const [yuan, setYuan] = useState("");
  const [credit, setCredit] = useState("");
  const [bonus, setBonus] = useState("0");
  const [note, setNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function create() {
    const amountFen = Math.round(Number(yuan) * 100);

    if (!name.trim() || !Number.isFinite(amountFen) || amountFen <= 0) {
      setNote("名称和金额要填对");
      return;
    }

    setBusy(true);
    setNote(null);

    try {
      await adminRequest("/api/v1/admin/recharge-plans", {
        method: "POST",
        body: {
          name: name.trim(),
          amount_fen: amountFen,
          credit: Number(credit) || 0,
          bonus: Number(bonus) || 0,
          sort: (plans.data ?? []).length
        }
      });

      setName("");
      setYuan("");
      setCredit("");
      setBonus("0");
      setNote("已添加");
      await plans.reload();
    } catch (err) {
      setNote(err instanceof Error ? err.message : "添加失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="panel">
      <h3>充值套餐</h3>
      <p className="hint">
        金额是用户实付，到账是充进钱包的额度，赠送在到账之外额外给。没有套餐时用户充不了值。
      </p>

      <div className="form-grid">
        <label>
          名称
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="例如：30 元档" />
        </label>
        <label>
          实付金额（元）
          <input value={yuan} onChange={(e) => setYuan(e.target.value)} placeholder="30" />
        </label>
        <label>
          到账额度
          <input value={credit} onChange={(e) => setCredit(e.target.value)} placeholder="300" />
        </label>
        <label>
          赠送额度
          <input value={bonus} onChange={(e) => setBonus(e.target.value)} placeholder="0" />
        </label>
      </div>

      {note && <p className="hint">{note}</p>}

      <button onClick={() => void create()} disabled={busy || !name || !yuan || !credit}>
        {busy ? "提交中…" : "添加套餐"}
      </button>

      <table>
        <thead>
          <tr>
            <th>名称</th>
            <th>实付</th>
            <th>到账</th>
            <th>赠送</th>
            <th>状态</th>
          </tr>
        </thead>
        <tbody>
          {(plans.data ?? []).map((item) => (
            <tr key={item.id}>
              <td>{item.name}</td>
              <td className="mono">¥{(item.amount_fen / 100).toFixed(2)}</td>
              <td className="mono">{item.credit}</td>
              <td className="mono">{item.bonus}</td>
              <td>{item.enabled ? "启用" : "停用"}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {(plans.data ?? []).length === 0 && <p className="hint">还没有套餐</p>}
    </div>
  );
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";

  const pad = (n: number) => String(n).padStart(2, "0");
  const day = date.getMonth() + 1 + "-" + pad(date.getDate());
  return day + " " + pad(date.getHours()) + ":" + pad(date.getMinutes());
}
