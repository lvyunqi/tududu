import { useState } from "react";

import UserAlbum from "../components/UserAlbum";
import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { AdminUser } from "../lib/types";
import { USER_STATUS } from "../lib/types";

/** 余额是定点小数，显示时去掉多余的 0。 */
function formatAmount(value: number): string {
  return String(Number(Number(value ?? 0).toFixed(4)));
}

function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  return `${(value / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export default function UsersPage() {
  const { data, error, loading, reload } = useLoad(() =>
    adminRequest<AdminUser[]>("/api/v1/admin/users?limit=50")
  );
  const [busy, setBusy] = useState<string | null>(null);
  const [album, setAlbum] = useState<AdminUser | null>(null);
  const [adjusting, setAdjusting] = useState<AdminUser | null>(null);

  async function changeStatus(user: AdminUser, status: number) {
    setBusy(user.id);

    try {
      await adminRequest(`/api/v1/admin/users/${user.id}/status`, {
        method: "PATCH",
        body: { status }
      });
      await reload();
    } finally {
      setBusy(null);
    }
  }

  if (loading) return <p className="hint">加载中…</p>;
  if (error) return <p className="error">{error}</p>;

  return (
    <section>
      <h2>用户</h2>
      <table>
        <thead>
          <tr>
            <th>编号</th>
            <th>昵称</th>
            <th>手机号</th>
            <th>余额</th>
            <th>占用</th>
            <th>状态</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {(data ?? []).map((user) => (
            <tr key={user.id}>
              <td className="mono">{user.short_code}</td>
              <td>{user.nickname}</td>
              <td className="mono">{user.phone ?? "—"}</td>
              <td className="mono">{formatAmount(user.balance)}</td>
              <td className="mono">{formatBytes(user.used_bytes)}</td>
              <td>{USER_STATUS[user.status] ?? user.status}</td>
              <td className="actions">
                <button className="small" onClick={() => setAlbum(user)}>
                  相册
                </button>
                <button className="small" onClick={() => setAdjusting(user)}>
                  改余额
                </button>
                {[1, 2, 3].map((status) => (
                  <button
                    key={status}
                    className="small"
                    disabled={busy === user.id || user.status === status}
                    onClick={() => void changeStatus(user, status)}
                  >
                    {USER_STATUS[status]}
                  </button>
                ))}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {(data ?? []).length === 0 && <p className="hint">还没有用户</p>}

      {album && (
        <UserAlbum
          userId={album.id}
          nickname={album.nickname}
          onClose={() => setAlbum(null)}
        />
      )}

      {adjusting && (
        <AdjustBalance
          user={adjusting}
          onClose={() => setAdjusting(null)}
          onDone={() => void reload()}
        />
      )}
    </section>
  );
}

/** 管理员加减余额：正数入账，负数扣减，都写流水。 */
function AdjustBalance({
  user,
  onClose,
  onDone
}: {
  user: AdminUser;
  onClose: () => void;
  onDone: () => void;
}) {
  const [amount, setAmount] = useState("");
  const [remark, setRemark] = useState("");
  const [note, setNote] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const value = Number(amount);
  const valid = Number.isFinite(value) && value !== 0;

  async function submit() {
    if (!valid) {
      setNote("填一个不为 0 的数，正数加，负数扣");
      return;
    }

    setBusy(true);
    setNote(null);

    try {
      await adminRequest(`/api/v1/admin/users/${user.id}/credits`, {
        method: "POST",
        body: { amount: value, remark: remark.trim() || undefined }
      });
      onDone();
      onClose();
    } catch (err) {
      setNote(err instanceof Error ? err.message : "调整失败");
      setBusy(false);
    }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal narrow" onClick={(event) => event.stopPropagation()}>
        <h3>调整余额 · {user.nickname}</h3>
        <p className="hint">当前余额 {formatAmount(user.balance)}</p>

        <div className="form-row">
          <input
            placeholder="调整数量，正数加、负数扣，支持小数"
            inputMode="decimal"
            value={amount}
            onChange={(event) => setAmount(event.target.value)}
          />
        </div>

        <div className="form-row">
          <input
            placeholder="备注，如 活动补偿"
            value={remark}
            onChange={(event) => setRemark(event.target.value)}
          />
        </div>

        {note && <p className="hint">{note}</p>}

        <div className="modal-actions">
          <button className="ghost" onClick={onClose} disabled={busy}>
            取消
          </button>
          <button onClick={() => void submit()} disabled={busy || !valid}>
            {busy ? "提交中…" : "确认调整"}
          </button>
        </div>
      </div>
    </div>
  );
}
