import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { AdminOrder } from "../lib/types";
import { ORDER_STATUS } from "../lib/types";

export default function OrdersPage() {
  const { data, error, loading } = useLoad(() =>
    adminRequest<AdminOrder[]>("/api/v1/admin/pay-orders?limit=50")
  );

  if (loading) return <p className="hint">加载中…</p>;
  if (error) return <p className="error">{error}</p>;

  return (
    <section>
      <h2>充值订单</h2>
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
          {(data ?? []).map((order) => (
            <tr key={order.order_no}>
              <td className="mono">{order.order_no}</td>
              <td className="mono">¥{(order.amount_fen / 100).toFixed(2)}</td>
              <td className="mono">
                {order.credit}
                {order.bonus > 0 ? ` + ${order.bonus}` : ""}
              </td>
              <td>{ORDER_STATUS[order.status] ?? order.status}</td>
              <td className="mono">{formatTime(order.created_at)}</td>
              <td className="mono">{order.paid_at ? formatTime(order.paid_at) : "—"}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {(data ?? []).length === 0 && <p className="hint">还没有订单</p>}
    </section>
  );
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getMonth() + 1}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(
    date.getMinutes()
  )}`;
}
