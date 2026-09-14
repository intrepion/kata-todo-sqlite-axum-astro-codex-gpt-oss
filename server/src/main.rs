use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = crate::db::init_db().await?;
    let shared_pool = Arc::new(pool);

    let app = Router::new()
        .nest("/api", crate::routes::router())
        .layer(axum::AddExtensionLayer::new(shared_pool))
        .layer(tower_http::cors::CorsLayer::new());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("🚀 Server listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
