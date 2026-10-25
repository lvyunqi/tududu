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
  const [note, setNote] = useState<string | null>(null);

  useEffect(() => {
    void request<RechargePlan[]>("/api/v1/billing/plans", { auth: false })
      .then(setPlans)
      .catch((err) => setError(err instanceof Error ? err.message : "加载套餐失败"));
  }, []);

  async function buy(plan: RechargePlan) {
    setBusy(true);
    setError(null);
    setNote(null);
    setStatus("正在下单…");

    try {
      const info = await request<CheckoutInfo>("/api/v1/billing/orders", {
        method: "POST",
        body: { plan_id: plan.id }
      });

      setPending(info);
      setStatus("订单已创建，点下面的按钮去付款");
      void watch(info.order_no);
    } catch (err) {
      setError(err instanceof Error ? err.message : "下单失败");
      setStatus(null);
    } finally {
      setBusy(false);
    }
  }

  /// 直接跳转。App 内 window.open 会被拦，同窗口跳转才可靠。
  function openPay() {
    if (!pending) return;
    window.location.href = pending.pay_url;
  }

  async function copyLink() {
    if (!pending) return;

    try {
      await navigator.clipboard.writeText(pending.pay_url);
      setNote("链接已复制，可以粘贴到浏览器打开");
    } catch {
      setNote("复制失败，长按下面的链接手动复制");
    }
  }

  async function checkNow() {
    if (!pending) return;
    setNote(null);

    try {
      const order = await request<PayOrder>("/api/v1/billing/orders/" + pending.order_no);
      if (order.status === 1) {
        setStatus("充值到账");
        await auth.refresh();
      } else {
        setNote("还没收到支付结果，稍等几秒再试");
      }
    } catch (err) {
      setNote(err instanceof Error ? err.message : "查询失败");
    }
  }

  async function watch(orderNo: string) {
    for (let i = 0; i < 40; i += 1) {
      await new Promise((resolve) => setTimeout(resolve, 3000));

      try {
        const order = await request<PayOrder>("/api/v1/billing/orders/" + orderNo);
        if (order.status === 1) {
          setStatus("充值到账");
          setPending(null);
          await auth.refresh();
          return;
        }
      } catch {
        return;
      }
    }
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
                {plan.bonus > 0 ? " 另赠 " + plan.bonus : ""}
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

      {pending && (
        <div className="pay-sheet">
          <h3>去支付</h3>
          <p className="hint">
            点「打开支付页面」会跳转到支付宝。付款完成后回到这里，会自己刷新余额。
          </p>

          <p className="hint mono">订单号 {pending.order_no}</p>

          <div className="pay-actions">
            <button onClick={openPay}>打开支付页面</button>
            <button className="ghost" onClick={() => void copyLink()}>
              复制链接
            </button>
          </div>

          <div className="pay-actions">
            <button className="ghost" onClick={() => void checkNow()}>
              我已付好，刷新状态
            </button>
            <button className="ghost" onClick={() => setPending(null)}>
              关闭
            </button>
          </div>

          {note && <p className="hint">{note}</p>}
          {pending.pay_url && <p className="hint mono break">{pending.pay_url}</p>}
        </div>
      )}
    </div>
  );
}
