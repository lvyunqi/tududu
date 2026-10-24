//! 后台定时任务：关闭过期订单、核对账本。

use std::time::Duration;

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// 两轮之间的间隔。这两件事都不急，五分钟一次足够。
const INTERVAL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Mismatch {
    pub user_id: Uuid,
    pub balance: i64,
    pub ledger_sum: i64,
}

#[derive(Debug, Serialize)]
pub struct ReconcileReport {
    pub expired_closed: u64,
    pub mismatches: Vec<Mismatch>,
}

/// 起一个后台循环。进程退出时随之结束，没有额外的关闭逻辑。
pub fn spawn(db: PgPool) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(INTERVAL);

        loop {
            ticker.tick().await;

            match run_once(&db).await {
                Ok(report) => {
                    if report.expired_closed > 0 {
                        tracing::info!(closed = report.expired_closed, "关闭过期订单");
                    }

                    if !report.mismatches.is_empty() {
                        tracing::error!(
                            count = report.mismatches.len(),
                            "账本与余额对不上，需要人工核查"
                        );
                    }
                }
                Err(err) => tracing::error!(error = %err, "后台任务执行失败"),
            }
        }
    });
}

/// 跑一轮：先关过期订单，再核对账本。
pub async fn run_once(db: &PgPool) -> sqlx::Result<ReconcileReport> {
    let expired_closed = close_expired_orders(db).await?;
    let mismatches = check_ledger(db).await?;

    Ok(ReconcileReport {
        expired_closed,
        mismatches,
    })
}

/// 过期的待支付订单标记为已关闭。已经支付的不会动。
pub async fn close_expired_orders(db: &PgPool) -> sqlx::Result<u64> {
    let result =
        sqlx::query("UPDATE pay_orders SET status = 2 WHERE status = 0 AND expire_at < now()")
            .execute(db)
            .await?;

    Ok(result.rows_affected())
}

/// 余额必须等于流水之和。对不上说明有人绕过接口改了余额，或者哪里写漏了。
pub async fn check_ledger(db: &PgPool) -> sqlx::Result<Vec<Mismatch>> {
    sqlx::query_as::<_, Mismatch>(
        "SELECT w.user_id, w.balance, COALESCE(SUM(l.amount), 0)::bigint AS ledger_sum \
           FROM wallets w LEFT JOIN ledger_entries l ON l.user_id = w.user_id \
          GROUP BY w.user_id, w.balance \
         HAVING w.balance <> COALESCE(SUM(l.amount), 0) \
          ORDER BY w.user_id LIMIT 100",
    )
    .fetch_all(db)
    .await
}
