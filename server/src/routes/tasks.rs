use axum::{
    extract::{Extension, Path, Json},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use crate::models::task::Task;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(list).post(create))
        .route("/:id", put(update).delete(delete))
}

async fn list(Extension(pool): Extension<Arc<SqlitePool>>) -> Result<impl IntoResponse, StatusCode> {
    let tasks = sqlx::query_as::<_, Task>("SELECT * FROM tasks")
        .fetch_all(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(tasks))
}

async fn create(
    Extension(pool): Extension<Arc<SqlitePool>>,
    Json(payload): Json<CreateTask>,
) -> Result<impl IntoResponse, StatusCode> {
    let rec = sqlx::query_as::<_, Task>(
        "INSERT INTO tasks (title, description) VALUES (?1, ?2) RETURNING *",
    )
    .bind(&payload.title)
    .bind(&payload.description)
    .fetch_one(&*pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(rec)))
}

async fn update(
    Extension(pool): Extension<Arc<SqlitePool>>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateTask>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = sqlx::query(
        "UPDATE tasks SET title = COALESCE(?1, title), description = COALESCE(?2, description), completed = COALESCE(?3, completed) WHERE id = ?4",
    )
    .bind(payload.title)
    .bind(payload.description)
    .bind(payload.completed)
    .bind(id)
    .execute(&*pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if res.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let updated = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?1")
        .bind(id)
        .fetch_one(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(updated))
}

async fn delete(
    Extension(pool): Extension<Arc<SqlitePool>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let res = sqlx::query("DELETE FROM tasks WHERE id = ?1")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if res.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
