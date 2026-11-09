import { useEffect, useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { BillingConfig, PriceRow } from "../lib/types";

export default function BillingPage() {
  const config = useLoad(() => adminRequest<BillingConfig>("/api/v1/admin/billing/config"));
  const prices = useLoad(() => adminRequest<PriceRow[]>("/api/v1/admin/billing/prices"));

  const [unitName, setUnitName] = useState("");
  const [unitPerYuan, setUnitPerYuan] = useState("");
  const [note, setNote] = useState<string | null>(null);

  const [model, setModel] = useState("");
  const [size, setSize] = useState("");
  const [quality, setQuality] = useState("");
  const [yuan, setYuan] = useState("");

  const [editing, setEditing] = useState<string | null>(null);
  const [editYuan, setEditYuan] = useState("");
  const [busy, setBusy] = useState(false);

  const perYuan = Number(unitPerYuan) || 0;

  useEffect(() => {
    if (config.data) {
      setUnitName(config.data.unit_name);
      setUnitPerYuan(String(config.data.unit_per_yuan));
    }
  }, [config.data]);

  async function saveConfig() {
    setNote(null);

    try {
      await adminRequest("/api/v1/admin/billing/config", {
        method: "PUT",
        body: { unit_name: unitName, unit_per_yuan: Math.round(Number(unitPerYuan)) }
      });
      await config.reload();
      setNote("计费单位已更新");
    } catch (err) {
      setNote(err instanceof Error ? err.message : "保存失败");
    }
  }

  /** 元换算成计费单位，四舍五入。 */
  function toUnits(value: string): number | null {
    const amount = Number(value);
    if (!Number.isFinite(amount) || amount < 0) return null;
    if (perYuan <= 0) return null;

    return Math.round(amount * perYuan);
  }

  function toYuan(units: number): string {
    if (perYuan <= 0) return "";

    const value = units / perYuan;
    // 去掉多余的 0，0.0500 -> 0.05
    return String(Number(value.toFixed(6)));
  }

  async function savePrice() {
    setNote(null);

    const units = toUnits(yuan);
    if (units === null) {
      setNote("先填好每元多少单位，再填一个不小于 0 的价格");
      return;
    }

    setBusy(true);
    try {
      await adminRequest("/api/v1/admin/billing/prices", {
        method: "PUT",
        body: { items: [{ model, size: size || null, quality: quality || null, price: units }] }
      });
      setModel("");
      setSize("");
      setQuality("");
      setYuan("");
      await prices.reload();
      setNote(`已保存：${yuan} 元 = ${units} ${unitName || "单位"}`);
    } catch (err) {
      setNote(err instanceof Error ? err.message : "保存失败");
    } finally {
      setBusy(false);
    }
  }

  function startEdit(row: PriceRow) {
    setEditing(row.id);
    setEditYuan(toYuan(row.price));
    setNote(null);
  }

  async function saveRow(row: PriceRow) {
    const units = toUnits(editYuan);
    if (units === null) {
      setNote("价格要填一个不小于 0 的数");
      return;
    }

    setBusy(true);
    try {
      await adminRequest("/api/v1/admin/billing/prices", {
        method: "PUT",
        body: {
          items: [{ model: row.model, size: row.size, quality: row.quality, price: units }]
        }
      });
      setEditing(null);
      await prices.reload();
      setNote("已更新 " + row.model);
    } catch (err) {
      setNote(err instanceof Error ? err.message : "保存失败");
    } finally {
      setBusy(false);
    }
  }

  const previewUnits = toUnits(yuan);

  return (
    <section>
      <h2>计费</h2>
      {note && <p className="hint">{note}</p>}

      <div className="panel">
        <h3>计费单位</h3>
        <p className="hint">
          1 元 = 多少{unitName || "单位"}。价格按元填写，按这个比例换算成{unitName || "单位"}保存；
          比例越大，能表达的价位越细。
        </p>
        <div className="form-row">
          <input placeholder="单位名称" value={unitName} onChange={(e) => setUnitName(e.target.value)} />
          <input
            placeholder="每元多少单位，如 100"
            value={unitPerYuan}
            onChange={(e) => setUnitPerYuan(e.target.value)}
          />
          <button onClick={() => void saveConfig()} disabled={!unitName || !unitPerYuan}>
            保存
          </button>
        </div>
      </div>

      <div className="panel">
        <h3>新增或更新价格</h3>
        <p className="hint">单价按元填写，支持小数，比如 0.05。</p>
        <div className="form-row">
          <input placeholder="模型，如 gpt-image-1" value={model} onChange={(e) => setModel(e.target.value)} />
          <input
            placeholder="尺寸，留空表示通配"
            value={size}
            onChange={(e) => setSize(e.target.value)}
          />
          <input
            placeholder="质量，留空表示通配"
            value={quality}
            onChange={(e) => setQuality(e.target.value)}
          />
          <input
            placeholder="单价（元），如 0.05"
            inputMode="decimal"
            value={yuan}
            onChange={(e) => setYuan(e.target.value)}
          />
          <button onClick={() => void savePrice()} disabled={busy || !model || !yuan}>
            保存
          </button>
        </div>
        {yuan && previewUnits !== null && (
          <p className="hint">
            = {previewUnits} {unitName || "单位"}
            {previewUnits === 0 && perYuan > 0 ? "（价格太小，换算后是 0，把每元单位调大一些）" : ""}
          </p>
        )}
      </div>

      <table>
        <thead>
          <tr>
            <th>模型</th>
            <th>尺寸</th>
            <th>质量</th>
            <th>单价</th>
            <th>启用</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {(prices.data ?? []).map((row) => (
            <tr key={row.id}>
              <td className="mono">{row.model}</td>
              <td className="mono">{row.size ?? "通配"}</td>
              <td className="mono">{row.quality ?? "通配"}</td>
              <td className="mono">
                {editing === row.id ? (
                  <input
                    className="price-input"
                    inputMode="decimal"
                    value={editYuan}
                    onChange={(e) => setEditYuan(e.target.value)}
                  />
                ) : (
                  `${toYuan(row.price)} 元 · ${row.price} ${unitName || "单位"}`
                )}
              </td>
              <td>{row.enabled ? "是" : "否"}</td>
              <td>
                {editing === row.id ? (
                  <span className="row-actions">
                    <button onClick={() => void saveRow(row)} disabled={busy}>
                      保存
                    </button>
                    <button className="ghost" onClick={() => setEditing(null)} disabled={busy}>
                      取消
                    </button>
                  </span>
                ) : (
                  <button onClick={() => startEdit(row)}>修改</button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {(prices.data ?? []).length === 0 && !prices.loading && <p className="hint">还没有价格</p>}
    </section>
  );
}
