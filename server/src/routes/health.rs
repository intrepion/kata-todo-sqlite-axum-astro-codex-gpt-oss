use axum::{routing::get, Router, Json};

pub fn router() -> Router {
    Router::new().route("/", get(|| async { Json(serde_json::json!({"status":"ok"})) }))
}
