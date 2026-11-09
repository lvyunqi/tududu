import { useEffect, useState } from "react";

import BackHome from "../components/BackHome";
import Toast from "../components/Toast";

import { BASE, getToken, request } from "../lib/api";
import type { useAuth } from "../lib/store";
import type { CheckoutInfo, PayOrder, RechargeOptions } from "../lib/types";

type Auth = ReturnType<typeof useAuth>;

/** 分转元，去掉多余的 0。 */
function yuan(fen: number): string {
  return String(Number((fen / 100).toFixed(2)));
}

export default function RechargePage({ auth }: { auth: Auth }) {
  const [options, setOptions] = useState<RechargeOptions | null>(null);
  const [amount, setAmount] = useState("");
  const [pending, setPending] = useState<CheckoutInfo | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const [qr, setQr] = useState<string | null>(null);

  useEffect(() => {
    void request<RechargeOptions>("/api/v1/billing/recharge-options", { auth: false })
      .then((data) => {
        setOptions(data);
        if (data.quick_amounts.length > 0) setAmount(yuan(data.quick_amounts[0]));
      })
      .catch((err) => setError(err instanceof Error ? err.message : "加载充值配置失败"));
  }, []);

  const perYuan = options?.unit_per_yuan ?? 0;
  const amountFen = Math.round(Number(amount || "0") * 100);
  const credit = perYuan > 0 && amountFen > 0 ? (amountFen / 100) * perYuan : 0;

  async function pay() {
    setBusy(true);
    setError(null);
    setNote(null);
    setStatus("正在下单…");

    try {
      const info = await request<CheckoutInfo>("/api/v1/billing/recharge", {
        method: "POST",
        body: { amount_fen: amountFen }
      });

      setPending(info);
      setStatus("订单已创建，扫码或在浏览器里打开都能付");
      setQr(null);
      void watch(info.order_no);
      void loadQr(info.pay_url);
    } catch (err) {
      setError(err instanceof Error ? err.message : "下单失败");
      setStatus(null);
    } finally {
      setBusy(false);
    }
  }

  /// 二维码由服务端渲染成 SVG，前端不引第三方库。
  async function loadQr(payUrl: string) {
    try {
      const response = await fetch(BASE + "/api/v1/qrcode", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: "Bearer " + (getToken() ?? "")
        },
        body: JSON.stringify({ text: payUrl })
      });

      if (!response.ok) return;

      const blob = await response.blob();
      setQr(URL.createObjectURL(blob));
    } catch {
      // 生成失败就不显示二维码，主按钮仍然可用
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
      setNote("复制失败，试试扫码");
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

  const canPay = amountFen >= 100 && credit > 0 && !busy;

  return (
    <div className="page">
      <header className="page-head">
        <BackHome />
        <h2>充值</h2>
        <span className="spacer" />
      </header>

      <p className="hint">
        当前余额 {auth.balance} {auth.unitName}
      </p>

      {error && <p className="error">{error}</p>}
      {status && <p className="hint">{status}</p>}

      <section className="card recharge-card">
        <label className="recharge-input">
          <span>充值金额</span>
          <div>
            <em>¥</em>
            <input
              inputMode="decimal"
              value={amount}
              onChange={(event) => setAmount(event.target.value.replace(/[^\d.]/g, ""))}
              placeholder="0"
            />
          </div>
        </label>

        {options && options.quick_amounts.length > 0 && (
          <div className="quick-amounts">
            {options.quick_amounts.map((fen) => (
              <button
                key={fen}
                className={Math.round(Number(amount || "0") * 100) === fen ? "chip on" : "chip"}
                onClick={() => setAmount(yuan(fen))}
              >
                ¥{yuan(fen)}
              </button>
            ))}
          </div>
        )}

        <p className="hint">
          {perYuan > 0
            ? `1 元 = ${perYuan} ${options?.unit_name ?? auth.unitName}，本次到账 ${Number(credit.toFixed(4))} ${options?.unit_name ?? auth.unitName}`
            : "正在读取充值比例…"}
        </p>

        <button className="block" onClick={() => void pay()} disabled={!canPay}>
          {busy ? "下单中…" : "去支付"}
        </button>
      </section>

      {pending && (
        <div className="pay-sheet">
          <h3>去支付</h3>
          <p className="hint">
            点「打开支付页面」会跳转到支付宝。付款完成后回到这里，会自己刷新余额。
          </p>

          <p className="hint mono">订单号 {pending.order_no}</p>
          <p className="hint">
            ¥{yuan(pending.amount_fen)} → {Number(pending.credit)} {options?.unit_name ?? auth.unitName}
          </p>

          {qr && (
            <div className="qr-box">
              <img src={qr} alt="支付二维码" />
              <p className="hint">截图后用支付宝扫一扫，或在浏览器里打开</p>
            </div>
          )}

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

          {note && <Toast text={note} onDone={() => setNote(null)} />}
        </div>
      )}
    </div>
  );
}
