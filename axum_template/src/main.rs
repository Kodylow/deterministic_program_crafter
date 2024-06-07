use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use axum::http::HeaderValue;
use axum::Json;
use db::{Database, Task};
use error::AppError;
use reqwest::StatusCode;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::info;

mod db;
mod error;

#[derive(Debug, Clone)]
struct AppState {
    db: Arc<Mutex<Database>>,
}

#[axum::debug_handler]
async fn create_task(
    State(app_state): State<AppState>,
    Json(task): Json<Task>,
) -> Result<Json<Task>, AppError> {
    let mut db = app_state.db.lock().await;
    db.insert_task(task.clone());

    // return a 200
    Ok(Json(task))
}

#[axum::debug_handler]
async fn read_task(
    State(app_state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Task>, AppError> {
    let db = app_state.db.lock().await;
    match db.get_task(id) {
        Some(task) => Ok(Json(task.clone())),
        None => Err(AppError::new(
            StatusCode::NOT_FOUND,
            anyhow::anyhow!("Task not found"),
        )),
    }
}

#[axum::debug_handler]
async fn read_tasks(State(app_state): State<AppState>) -> Result<Json<Vec<Task>>, AppError> {
    let db = app_state.db.lock().await;
    let tasks = db.get_tasks();
    Ok(Json(tasks))
}

#[axum::debug_handler]
async fn update_task(
    State(app_state): State<AppState>,
    Path(id): Path<u64>,
    Json(task): Json<Task>,
) -> Result<Json<Task>, AppError> {
    let mut db = app_state.db.lock().await;
    db.update_task(id, task.clone());
    Ok(Json(task))
}

#[axum::debug_handler]
async fn delete_task(
    State(app_state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<()>, AppError> {
    let mut db = app_state.db.lock().await;
    db.delete_task(id);
    Ok(Json(()))
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
        .route("/task", axum::routing::post(create_task))
        .route("/tasks", axum::routing::get(read_tasks))
        .route("/task/:id", axum::routing::get(read_task))
        .route("/task/:id", axum::routing::put(update_task))
        .route("/task/:id", axum::routing::delete(delete_task))
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
