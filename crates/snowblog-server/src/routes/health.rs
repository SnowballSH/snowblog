use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::problem::Problem;
use crate::state::AppState;

pub async fn health(State(state): State<AppState>) -> Result<Json<Value>, Problem> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(state.service.store().pool())
        .await
        .map_err(|e| Problem::internal(format!("database unreachable: {e}")))?;
    Ok(Json(json!({
        "service_version": env!("CARGO_PKG_VERSION"),
        "renderer_version": state.service.renderer().version(),
        "database": "ok",
    })))
}
