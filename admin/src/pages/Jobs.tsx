import { useState } from "react";

import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { AdminJob, JobDetail } from "../lib/types";
import { JOB_STATUS } from "../lib/types";

export default function JobsPage() {
  const { data, error, loading } = useLoad(() =>
    adminRequest<AdminJob[]>("/api/v1/admin/jobs?limit=50")
  );
  const [detail, setDetail] = useState<JobDetail | null>(null);

  async function open(job: AdminJob) {
    const found = await adminRequest<JobDetail>(`/api/v1/admin/jobs/${job.id}`);
    setDetail(found);
  }

  if (loading) return <p className="hint">加载中…</p>;
  if (error) return <p className="error">{error}</p>;

  return (
    <section>
      <h2>任务</h2>
      <table>
        <thead>
          <tr>
            <th>任务号</th>
            <th>模型</th>
            <th>状态</th>
            <th>扣费</th>
            <th>错误</th>
            <th>创建时间</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {(data ?? []).map((job) => (
            <tr key={job.id}>
              <td className="mono">{job.id.slice(0, 8)}</td>
              <td>{job.model}</td>
              <td>{JOB_STATUS[job.status] ?? job.status}</td>
              <td className="mono">{job.price}</td>
              <td className="error-cell">{job.error_message ?? "—"}</td>
              <td className="mono">{job.created_at.slice(5, 16).replace("T", " ")}</td>
              <td>
                <button className="small" onClick={() => void open(job)}>
                  详情
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {(data ?? []).length === 0 && <p className="hint">还没有任务</p>}

      {detail && (
        <div className="drawer">
          <div className="drawer-head">
            <h3>任务 {detail.job.id.slice(0, 8)}</h3>
            <button className="small" onClick={() => setDetail(null)}>
              关闭
            </button>
          </div>

          <table>
            <thead>
              <tr>
                <th>第几次</th>
                <th>结果</th>
                <th>HTTP</th>
                <th>耗时</th>
                <th>错误码</th>
              </tr>
            </thead>
            <tbody>
              {detail.attempts.map((attempt) => (
                <tr key={attempt.attempt_no}>
                  <td className="mono">{attempt.attempt_no}</td>
                  <td>{attemptText(attempt.status)}</td>
                  <td className="mono">{attempt.http_status ?? "—"}</td>
                  <td className="mono">
                    {attempt.latency_ms != null ? `${attempt.latency_ms} ms` : "—"}
                  </td>
                  <td className="mono">{attempt.error_code ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function attemptText(status: number): string {
  if (status === 1) return "成功";
  if (status === 2) return "可重试失败";
  return "不可重试";
}
