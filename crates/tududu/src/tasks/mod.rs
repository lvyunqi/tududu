//! 后台定时任务：关闭过期订单、核对账本。

use std::time::Duration;

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::job::store as job_store;
use crate::session::store as session_store;
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
    pub recovered_jobs: u64,
    pub mismatches: Vec<Mismatch>,
}

/// 起一个后台循环。进程退出时随之结束，没有额外的关闭逻辑。
pub fn spawn(
    db: PgPool,
    files: LocalStore,
    settings: std::sync::Arc<std::sync::RwLock<crate::settings::LiveSettings>>,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(INTERVAL);

        loop {
            ticker.tick().await;

            // 每轮都读一次，后台改完保留天数下一轮就生效
            let retention_days = settings
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .trash_retention_days;

            match run_once(&db, &files, retention_days).await {
                Ok(report) => {
                    if report.expired_closed > 0 {
                        tracing::info!(closed = report.expired_closed, "关闭过期订单");
                    }

                    if report.purged > 0 {
                        tracing::info!(purged = report.purged, "清理回收站");
                    }

                    if report.recovered_jobs > 0 {
                        tracing::warn!(count = report.recovered_jobs, "回收没有进程在跑的任务");
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
    let recovered_jobs = recover_orphan_jobs(db, ORPHAN_GRACE_SECS).await?;
    let mismatches = check_ledger(db).await?;

    Ok(ReconcileReport {
        expired_closed,
        purged,
        recovered_jobs,
        mismatches,
    })
}

/// 定期扫描时给任务留的余量。正常任务最多几分钟，半小时还没结束肯定是孤儿。
pub const ORPHAN_GRACE_SECS: i64 = 30 * 60;

/// 把没有进程在跑的任务标成失败并退款。
///
/// 任务跑在进程内，服务一重启就全丢了，但库里状态还停在执行中，用户的钱也还扣着。
/// 启动时调用时传 0，因为刚起来的进程不可能有正在跑的任务。
pub async fn recover_orphan_jobs(db: &PgPool, older_than_secs: i64) -> sqlx::Result<u64> {
    let orphans = job_store::find_orphans(db, older_than_secs).await?;
    let mut recovered = 0_u64;

    for job in orphans {
        job_store::mark_failed(db, job.id, 30006, "服务重启，任务已中止，费用已退回").await?;

        if let Some(message_id) = job.message_id {
            let outcome =
                session_store::finish_message(db, message_id, session_store::MSG_FAILED, None)
                    .await;

            if let Err(err) = outcome {
                tracing::warn!(error = %err, %message_id, "回写会话消息失败");
            }
        }

        if job.price > 0 {
            let outcome = crate::billing::refund(
                db,
                job.user_id,
                job.price,
                Some("job"),
                Some(job.id),
                "任务中止退回",
            )
            .await;

            if let Err(err) = outcome {
                tracing::error!(error = %err, job_id = %job.id, "退回失败，需要人工核查");
            }
        }

        recovered += 1;
    }

    Ok(recovered)
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
