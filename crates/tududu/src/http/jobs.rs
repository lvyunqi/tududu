use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use uuid::Uuid;

use crate::error::{ApiBody, ApiError};
use crate::http::auth::require_user;
use crate::http::routes::AppState;
use crate::job::{self, CreateJobRequest};

/// 提交任务后立刻返回，真正的上游调用放到后台跑。
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateJobRequest>,
) -> Result<Json<ApiBody<job::store::JobRow>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    limit_jobs(&state, current.user_id)?;
    let created = job::submit(&state.db, current.user_id, req, job::JobLink::default()).await?;

    let background = state.clone();
    let job_id = created.id;
    tokio::spawn(async move {
        let outcome = job::runner::execute(
            &background.db,
            &background.files,
            &background.http,
            &background.provider_key,
            job_id,
        )
        .await;

        if let Err(err) = outcome {
            tracing::error!(error = %err, %job_id, "任务执行异常");
        }
    });

    Ok(Json(ApiBody::ok(created)))
}

/// 单用户一小时最多提交三十个任务。
fn limit_jobs(state: &AppState, user_id: Uuid) -> Result<(), ApiError> {
    let key = format!("job:{user_id}");

    state
        .limiter
        .check(&key, 30, std::time::Duration::from_secs(3600))
        .map_err(|wait| {
            ApiError::too_many_requests(format!(
                "提交太频繁，请 {} 分钟后再试",
                (wait.as_secs() / 60).max(1)
            ))
        })
}

pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiBody<job::store::JobRow>>, ApiError> {
    let current = require_user(&state, &headers).await?;
    let found = job::get(&state.db, current.user_id, id).await?;
    Ok(Json(ApiBody::ok(found)))
}
