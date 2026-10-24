//! 后台定时任务：关闭过期订单、核对账本。

use std::time::Duration;

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::storage::local_store::LocalStore;

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
    pub purged: u64,
    pub mismatches: Vec<Mismatch>,
}

/// 起一个后台循环。进程退出时随之结束，没有额外的关闭逻辑。
pub fn spawn(db: PgPool, files: LocalStore, retention_days: i32) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(INTERVAL);

        loop {
            ticker.tick().await;

            match run_once(&db, &files, retention_days).await {
                Ok(report) => {
                    if report.expired_closed > 0 {
                        tracing::info!(closed = report.expired_closed, "关闭过期订单");
                    }

                    if report.purged > 0 {
                        tracing::info!(purged = report.purged, "清理回收站");
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

/// 跑一轮：关过期订单、清回收站、核对账本。
pub async fn run_once(
    db: &PgPool,
    files: &LocalStore,
    retention_days: i32,
) -> sqlx::Result<ReconcileReport> {
    let expired_closed = close_expired_orders(db).await?;
    let purged = purge_trash(db, files, retention_days).await?;
    let mismatches = check_ledger(db).await?;

    Ok(ReconcileReport {
        expired_closed,
        purged,
        mismatches,
    })
}

/// 把超过保留期的回收站条目真正清掉：先删文件，再把状态标成已删除。
///
/// 文件删除失败只记日志，不影响状态推进——否则一条删不掉的记录会永远卡在队列里。
pub async fn purge_trash(
    db: &PgPool,
    files: &LocalStore,
    retention_days: i32,
) -> sqlx::Result<u64> {
    let rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
        "SELECT id, object_key, thumb_key FROM assets \
          WHERE status = 2 AND trashed_at IS NOT NULL \
            AND trashed_at < now() - make_interval(days => $1) \
          ORDER BY trashed_at LIMIT 200",
    )
    .bind(retention_days)
    .fetch_all(db)
    .await?;

    let mut purged = 0_u64;

    for (id, object_key, thumb_key) in rows {
        if let Err(err) = files.remove(&object_key).await {
            tracing::warn!(error = %err, key = %object_key, "删除原图失败");
        }

        if let Some(key) = thumb_key.as_deref()
            && let Err(err) = files.remove(key).await
        {
            tracing::warn!(error = %err, key = %key, "删除缩略图失败");
        }

        sqlx::query("UPDATE assets SET status = 3 WHERE id = $1 AND status = 2")
            .bind(id)
            .execute(db)
            .await?;

        purged += 1;
    }

    Ok(purged)
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
