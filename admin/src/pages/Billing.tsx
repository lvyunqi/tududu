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
  const [price, setPrice] = useState("");

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
        body: { unit_name: unitName, unit_per_yuan: Number(unitPerYuan) }
      });
      await config.reload();
      setNote("计费单位已更新");
    } catch (err) {
      setNote(err instanceof Error ? err.message : "保存失败");
    }
  }

  async function savePrice() {
    setNote(null);

    try {
      await adminRequest("/api/v1/admin/billing/prices", {
        method: "PUT",
        body: {
          items: [
            {
              model,
              size: size || null,
              quality: quality || null,
              price: Number(price)
            }
          ]
        }
      });
      setModel("");
      setSize("");
      setQuality("");
      setPrice("");
      await prices.reload();
      setNote("价格已保存");
    } catch (err) {
      setNote(err instanceof Error ? err.message : "保存失败");
    }
  }

  return (
    <section>
      <h2>计费</h2>
      {note && <p className="hint">{note}</p>}

      <div className="panel">
        <h3>计费单位</h3>
        <div className="form-row">
          <input placeholder="单位名称" value={unitName} onChange={(e) => setUnitName(e.target.value)} />
          <input
            placeholder="每元多少单位"
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
          <input placeholder="单价" value={price} onChange={(e) => setPrice(e.target.value)} />
          <button onClick={() => void savePrice()} disabled={!model || !price}>
            保存
          </button>
        </div>
      </div>

      <table>
        <thead>
          <tr>
            <th>模型</th>
            <th>尺寸</th>
            <th>质量</th>
            <th>单价</th>
            <th>启用</th>
          </tr>
        </thead>
        <tbody>
          {(prices.data ?? []).map((row) => (
            <tr key={row.id}>
              <td className="mono">{row.model}</td>
              <td className="mono">{row.size ?? "通配"}</td>
              <td className="mono">{row.quality ?? "通配"}</td>
              <td className="mono">{row.price}</td>
              <td>{row.enabled ? "是" : "否"}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {(prices.data ?? []).length === 0 && !prices.loading && <p className="hint">还没有价格</p>}
    </section>
  );
}
