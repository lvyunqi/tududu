import { useEffect, useState } from "react";

import { request } from "../lib/api";
import type { useAuth } from "../lib/store";
import type { CheckoutInfo, PayOrder, RechargePlan } from "../lib/types";

type Auth = ReturnType<typeof useAuth>;

export default function RechargePage({ auth }: { auth: Auth }) {
  const [plans, setPlans] = useState<RechargePlan[]>([]);
  const [pending, setPending] = useState<CheckoutInfo | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void request<RechargePlan[]>("/api/v1/billing/plans", { auth: false })
      .then(setPlans)
      .catch((err) => setError(err instanceof Error ? err.message : "加载套餐失败"));
  }, []);

  async function buy(plan: RechargePlan) {
    setBusy(true);
    setError(null);
    setStatus("正在下单…");

    try {
      const info = await request<CheckoutInfo>("/api/v1/billing/orders", {
        method: "POST",
        body: { plan_id: plan.id }
      });

      setPending(info);
      setStatus("已打开支付页面，完成后回到这里等结果");
      window.open(info.pay_url, "_blank", "noopener,noreferrer");
      void watch(info.order_no);
    } catch (err) {
      setError(err instanceof Error ? err.message : "下单失败");
      setStatus(null);
    } finally {
      setBusy(false);
    }
  }

  async function watch(orderNo: string) {
    for (let i = 0; i < 40; i += 1) {
      await new Promise((resolve) => setTimeout(resolve, 3000));

      try {
        const order = await request<PayOrder>(`/api/v1/billing/orders/${orderNo}`);
        if (order.status === 1) {
          setStatus("充值到账");
          await auth.refresh();
          return;
        }
      } catch {
        return;
      }
    }

    setStatus("还没收到支付结果，稍后可在订单里核对");
  }

  return (
    <div className="page">
      <a className="link back" href="#/me">
        返回
      </a>

      <h2>充值</h2>
      <p className="hint">
        当前余额 {auth.balance} {auth.unitName}
      </p>

      {error && <p className="error">{error}</p>}
      {status && <p className="hint">{status}</p>}

      <div className="list">
        {plans.map((plan) => (
          <article key={plan.id} className="card plan">
            <div>
              <h3>{plan.name}</h3>
              <p className="hint">
                {plan.credit} {auth.unitName}
                {plan.bonus > 0 ? ` 另赠 ${plan.bonus}` : ""}
              </p>
            </div>
            <div className="plan-right">
              <span className="price">¥{(plan.amount_fen / 100).toFixed(2)}</span>
              <button onClick={() => void buy(plan)} disabled={busy}>
                购买
              </button>
            </div>
          </article>
        ))}
      </div>

      {plans.length === 0 && !error && <p className="hint">还没有可购买的套餐</p>}
      {pending && <p className="hint">订单号 {pending.order_no}</p>}
    </div>
  );
}
