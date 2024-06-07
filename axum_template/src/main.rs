use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use axum::http::HeaderValue;
use db::Database;
use reqwest::Client as ReqwestClient;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::info;

mod db;
mod error;
mod handlers;

#[derive(Debug, Clone)]
struct AppState {
    db: Arc<Mutex<Database>>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_logging_and_env()?;
    let db = Arc::new(Mutex::new(Database::load_or_create()?));
    let app_state = AppState { db };
    let cors = CorsLayer::new()
        .allow_origin(HeaderValue::from_static("http://127.0.0.1:8080"))
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT])
        .max_age(Duration::from_secs(3600));

    let app = axum::Router::new()
        .route("/task", axum::routing::post(handlers::create_task))
        .route("/tasks", axum::routing::get(handlers::read_tasks))
        .route("/task/:id", axum::routing::get(handlers::read_task))
        .route("/task/:id", axum::routing::put(handlers::update_task))
        .route("/task/:id", axum::routing::delete(handlers::delete_task))
        .route("/register", axum::routing::post(handlers::create_user))
        .route("/login", axum::routing::post(handlers::login))
        .layer(cors)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to port 8080: {e}"))?;
    info!("Server listening on http://127.0.0.1:8080");

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start server: {e}"))?;

    Ok(())
}

fn init_logging_and_env() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    Ok(())
}
