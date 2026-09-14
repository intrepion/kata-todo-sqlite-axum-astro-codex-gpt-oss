mod tasks;
mod health;

pub fn router() -> axum::Router {
    axum::Router::new()
        .nest("/tasks", tasks::router())
        .nest("/", health::router())
}
