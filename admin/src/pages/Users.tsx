import { useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { AdminUser } from "../lib/types";
import { USER_STATUS } from "../lib/types";

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
              <td className="mono">{user.balance}</td>
              <td className="mono">{formatBytes(user.used_bytes)}</td>
              <td>{USER_STATUS[user.status] ?? user.status}</td>
              <td className="actions">
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
    </section>
  );
}
