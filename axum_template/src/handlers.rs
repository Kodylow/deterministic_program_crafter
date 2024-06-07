use axum::extract::{Path, State};
use axum::Json;
use reqwest::StatusCode;

use crate::db::{Task, User};
use crate::error::AppError;
use crate::AppState;

#[axum::debug_handler]
pub async fn create_task(
    State(app_state): State<AppState>,
    Json(task): Json<Task>,
) -> Result<Json<Task>, AppError> {
    let mut db = app_state.db.lock().await;
    db.insert_task(task.clone());

    // return a 200
    Ok(Json(task))
}

#[axum::debug_handler]
pub async fn read_task(
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
pub async fn read_tasks(State(app_state): State<AppState>) -> Result<Json<Vec<Task>>, AppError> {
    let db = app_state.db.lock().await;
    let tasks = db.get_tasks();
    Ok(Json(tasks))
}

#[axum::debug_handler]
pub async fn update_task(
    State(app_state): State<AppState>,
    Path(id): Path<u64>,
    Json(task): Json<Task>,
) -> Result<Json<Task>, AppError> {
    let mut db = app_state.db.lock().await;
    db.update_task(id, task.clone());
    Ok(Json(task))
}

#[axum::debug_handler]
pub async fn delete_task(
    State(app_state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<()>, AppError> {
    let mut db = app_state.db.lock().await;
    db.delete_task(id);
    Ok(Json(()))
}

#[axum::debug_handler]
pub async fn create_user(
    State(app_state): State<AppState>,
    Json(user): Json<User>,
) -> Result<Json<User>, AppError> {
    let mut db = app_state.db.lock().await;
    db.insert_user(user.clone());

    // return a 200
    Ok(Json(user))
}

#[axum::debug_handler]
pub async fn login(
    State(app_state): State<AppState>,
    Json(user): Json<User>,
) -> Result<Json<String>, AppError> {
    let db = app_state.db.lock().await;
    let user = db.get_user_by_username(&user.username);
    match user {
        Some(user) => {
            if user.password == user.password {
                return Ok(Json(user.username));
            } else {
                return Err(AppError::new(
                    StatusCode::UNAUTHORIZED,
                    anyhow::anyhow!("Invalid username or password"),
                ));
            }
        }
        None => {
            return Err(AppError::new(
                StatusCode::UNAUTHORIZED,
                anyhow::anyhow!("Invalid username or password"),
            ))
        }
    }
}
