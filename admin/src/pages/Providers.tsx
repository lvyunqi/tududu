import { useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { Channel, Model, Provider } from "../lib/types";

export default function ProvidersPage() {
  const list = useLoad(() => adminRequest<Provider[]>("/api/v1/admin/providers"));
  const [selected, setSelected] = useState<Provider | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");

  const [channelName, setChannelName] = useState("");
  const [apiKey, setApiKey] = useState("");

  async function createProvider() {
    setMessage(null);

    try {
      await adminRequest("/api/v1/admin/providers", {
        method: "POST",
        body: { code, name, base_url: baseUrl }
      });
      setCode("");
      setName("");
      setBaseUrl("");
      await list.reload();
      setMessage("渠道商已添加");
    } catch (err) {
      setMessage(err instanceof Error ? err.message : "添加失败");
    }
  }

  return (
    <section>
      <h2>渠道</h2>
      {message && <p className="hint">{message}</p>}

      <div className="panel">
        <h3>新增渠道商</h3>
        <div className="form-row">
          <input placeholder="标识，如 openai" value={code} onChange={(e) => setCode(e.target.value)} />
          <input placeholder="名称" value={name} onChange={(e) => setName(e.target.value)} />
          <input
            placeholder="接口地址，如 https://api.openai.com/v1"
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
          />
          <button onClick={() => void createProvider()} disabled={!code || !name || !baseUrl}>
            添加
          </button>
        </div>
      </div>

      <table>
        <thead>
          <tr>
            <th>标识</th>
            <th>名称</th>
            <th>地址</th>
            <th>超时</th>
            <th>并发</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {(list.data ?? []).map((item) => (
            <tr key={item.id}>
              <td className="mono">{item.code}</td>
              <td>{item.name}</td>
              <td className="mono">{item.base_url}</td>
              <td className="mono">{item.timeout_ms} ms</td>
              <td className="mono">{item.max_concurrency}</td>
              <td>
                <button className="small" onClick={() => setSelected(item)}>
                  密钥与模型
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {(list.data ?? []).length === 0 && !list.loading && <p className="hint">还没有渠道商</p>}

      {selected && (
        <ProviderDetail
          provider={selected}
          channelName={channelName}
          apiKey={apiKey}
          onChannelName={setChannelName}
          onApiKey={setApiKey}
          onClose={() => setSelected(null)}
        />
      )}
    </section>
  );
}

function ProviderDetail({
  provider,
  channelName,
  apiKey,
  onChannelName,
  onApiKey,
  onClose
}: {
  provider: Provider;
  channelName: string;
  apiKey: string;
  onChannelName: (value: string) => void;
  onApiKey: (value: string) => void;
  onClose: () => void;
}) {
  const channels = useLoad(() =>
    adminRequest<Channel[]>(`/api/v1/admin/providers/${provider.id}/channels`)
  );
  const models = useLoad(() => adminRequest<Model[]>(`/api/v1/admin/providers/${provider.id}/models`));
  const [note, setNote] = useState<string | null>(null);

  async function addChannel() {
    setNote(null);

    try {
      await adminRequest(`/api/v1/admin/providers/${provider.id}/channels`, {
        method: "POST",
        body: { name: channelName, api_key: apiKey }
      });
      onChannelName("");
      onApiKey("");
      await channels.reload();
      setNote("密钥已保存");
    } catch (err) {
      setNote(err instanceof Error ? err.message : "保存失败");
    }
  }

  async function refreshModels() {
    setNote(null);

    try {
      await adminRequest(`/api/v1/admin/providers/${provider.id}/models/refresh`, {
        method: "POST"
      });
      await models.reload();
      setNote("模型清单已更新");
    } catch (err) {
      setNote(err instanceof Error ? err.message : "拉取失败");
    }
  }

  return (
    <div className="drawer">
      <div className="drawer-head">
        <h3>{provider.name}</h3>
        <button className="small" onClick={onClose}>
          关闭
        </button>
      </div>

      {note && <p className="hint">{note}</p>}

      <h4>接入密钥</h4>
      <table>
        <thead>
          <tr>
            <th>名称</th>
            <th>密钥</th>
            <th>优先级</th>
            <th>健康</th>
          </tr>
        </thead>
        <tbody>
          {(channels.data ?? []).map((item) => (
            <tr key={item.id}>
              <td>{item.name}</td>
              <td className="mono">{item.api_key_hint}</td>
              <td className="mono">{item.priority}</td>
              <td className="mono">{item.health}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <div className="form-row">
        <input
          placeholder="密钥名称"
          value={channelName}
          onChange={(e) => onChannelName(e.target.value)}
        />
        <input
          placeholder="sk-..."
          value={apiKey}
          onChange={(e) => onApiKey(e.target.value)}
        />
        <button onClick={() => void addChannel()} disabled={!channelName || !apiKey}>
          保存密钥
        </button>
      </div>

      <h4>模型清单</h4>
      <div className="form-row">
        <button onClick={() => void refreshModels()}>从上游拉取</button>
      </div>
      <p className="hint">
        {(models.data ?? []).map((item) => item.code).join("、") || "还没有模型"}
      </p>
    </div>
  );
}
