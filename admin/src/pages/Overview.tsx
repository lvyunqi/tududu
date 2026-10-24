import { useLoad } from "../components/useLoad";
import { adminRequest } from "../lib/api";
import type { Overview } from "../lib/types";

export default function OverviewPage() {
  const { data, error, loading } = useLoad(() => adminRequest<Overview>("/api/v1/admin/overview"));

  if (loading) return <p className="hint">加载中…</p>;
  if (error) return <p className="error">{error}</p>;
  if (!data) return null;

  const cards = [
    { label: "用户总数", value: data.users_total },
    { label: "今日新增", value: data.users_today },
    { label: "今日充值笔数", value: data.paid_orders_today },
    { label: "今日充值金额", value: `¥${(data.revenue_fen_today / 100).toFixed(2)}` },
    { label: "今日任务", value: data.jobs_today },
    { label: "今日失败", value: data.jobs_failed_today },
    { label: "已发布作品", value: data.prompts_published }
  ];

  return (
    <section>
      <h2>看板</h2>
      <div className="cards">
        {cards.map((item) => (
          <div key={item.label} className="metric">
            <span className="hint">{item.label}</span>
            <strong>{item.value}</strong>
          </div>
        ))}
      </div>
    </section>
  );
}
