//! # Module 07: Error Handling
//!
//! Proper error handling in Axum:
//! - Custom error types with thiserror
//! - IntoResponse for errors
//! - Result-based handlers
//! - Error recovery patterns

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use thiserror::Error;

// ============================================================================
// LESSON 1: Custom Error Types with thiserror
// ============================================================================

#[derive(Error, Debug)]
#[allow(dead_code)] // Variants shown for demonstration
enum AppError {
    #[error("User not found: {0}")]
    UserNotFound(u64),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Internal server error")]
    Internal,
}

// ============================================================================
// LESSON 2: Implement IntoResponse for Custom Error
// ============================================================================

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: u16,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::UserNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::InvalidInput(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = ErrorResponse {
            error: message,
            code: status.as_u16(),
        };

        (status, Json(body)).into_response()
    }
}

// ============================================================================
// LESSON 3: Result-Based Handlers
// ============================================================================

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

async fn get_user(Path(id): Path<u64>) -> Result<Json<User>, AppError> {
    // Simulated user lookup
    match id {
        1 => Ok(Json(User {
            id: 1,
            name: "Alice".to_string(),
        })),
        2 => Ok(Json(User {
            id: 2,
            name: "Bob".to_string(),
        })),
        _ => Err(AppError::UserNotFound(id)),
    }
}

async fn validate_input(Path(value): Path<String>) -> Result<String, AppError> {
    if value.len() < 3 {
        return Err(AppError::InvalidInput(
            "Value must be at least 3 characters".to_string(),
        ));
    }
    Ok(format!("Valid input: {}", value))
}

async fn protected_resource() -> Result<&'static str, AppError> {
    // Simulated auth check
    let is_authenticated = false;
    if !is_authenticated {
        return Err(AppError::Unauthorized);
    }
    Ok("Secret data!")
}

async fn database_operation() -> Result<&'static str, AppError> {
    // Simulated database error
    Err(AppError::DatabaseError("Connection timeout".to_string()))
}

// ============================================================================
// LESSON 4: Fallible Operations with ?
// ============================================================================

async fn complex_operation(Path(id): Path<u64>) -> Result<Json<User>, AppError> {
    // Use ? operator for early returns
    let user = find_user(id)?;
    validate_user(&user)?;
    Ok(Json(user))
}

fn find_user(id: u64) -> Result<User, AppError> {
    if id == 0 {
        Err(AppError::InvalidInput("ID cannot be zero".to_string()))
    } else if id > 100 {
        Err(AppError::UserNotFound(id))
    } else {
        Ok(User {
            id,
            name: format!("User{}", id),
        })
    }
}

fn validate_user(user: &User) -> Result<(), AppError> {
    if user.name.is_empty() {
        Err(AppError::InvalidInput("Name cannot be empty".to_string()))
    } else {
        Ok(())
    }
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/users/{id}", get(get_user))
        .route("/validate/{value}", get(validate_input))
        .route("/protected", get(protected_resource))
        .route("/database", get(database_operation))
        .route("/complex/{id}", get(complex_operation));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("🚀 Module 07: Error Handling");
    println!("   Server: http://localhost:3000\n");
    println!("📝 Try these endpoints:");
    println!("   GET /users/1      - Success (user exists)");
    println!("   GET /users/999    - 404 (user not found)");
    println!("   GET /validate/ab  - 400 (too short)");
    println!("   GET /protected    - 401 (unauthorized)");
    println!("   GET /database     - 500 (database error)");

    axum::serve(listener, app).await.unwrap();
}
