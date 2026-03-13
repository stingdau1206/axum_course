use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool};
use std::sync::Arc;
use uuid::Uuid;
use dotenvy::dotenv;

// ============================================================================
// MODELS & CONFIG
// ============================================================================

#[derive(Clone)]
struct AuthConfig {
    jwt_secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // user_id
    exp: usize,
    role: String,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Clone)]
struct User {
    id: Uuid,
    username: String,
    password: String, // Đây là chuỗi đã băm
    is_lock: bool,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CurrentUser {
    id: String,
    role: String,
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Database error")]
    Sqlx(#[from] sqlx::Error),
    #[error("User not found or wrong password")]
    Unauthorized,
    #[error("Internal error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::Sqlx(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
            AppError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };
        (status, msg).into_response()
    }
}

// ============================================================================
// AUTH UTILS (Băm & JWT)
// ============================================================================

fn hash_password(password: &str) -> String {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut rand::thread_rng());
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let parsed_hash = PasswordHash::new(hash).unwrap();
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

fn create_token(config: &AuthConfig, user_id: &str) -> Result<String, AppError> {
    let expiry = Utc::now() + Duration::hours(24);
    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiry.timestamp() as usize,
        role: "user".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|_| AppError::Internal)
}

// ============================================================================
// HANDLERS
// ============================================================================

async fn register(State(pool): State<PgPool>, Json(payload): Json<LoginRequest>) -> Result<StatusCode, AppError> {
    let hashed = hash_password(&payload.password);
    sqlx::query("INSERT INTO users (id, username, password, is_lock) VALUES ($1, $2, $3, $4)")
        .bind(Uuid::new_v4())
        .bind(&payload.username)
        .bind(&hashed)
        .bind(false)
        .execute(&pool)
        .await?;
    Ok(StatusCode::CREATED)
}

async fn login(
    State(pool): State<PgPool>,
    State(config): State<Arc<AuthConfig>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    // 1. Tìm user
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(&pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // 2. Verify pass
    if !verify_password(&payload.password, &user.password) {
        return Err(AppError::Unauthorized);
    }

    // 3. Tạo Token
    let token = create_token(&config, &user.id.to_string())?;
    Ok(Json(LoginResponse { token }))
}

// Route được bảo vệ
async fn get_me(axum::Extension(user): axum::Extension<CurrentUser>) -> Json<CurrentUser> {
    Json(user)
}

// ============================================================================
// AUTH MIDDLEWARE
// ============================================================================

async fn auth_middleware(
    State(config): State<Arc<AuthConfig>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let token = auth_header.ok_or(StatusCode::UNAUTHORIZED)?;
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let current_user = CurrentUser {
        id: token_data.claims.sub,
        role: token_data.claims.role,
    };

    request.extensions_mut().insert(current_user); // Bơm thông tin user vào request
    Ok(next.run(request).await)
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() {
    dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Can't connect to DB");

    // Khởi tạo bảng
    sqlx::query("
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password TEXT NOT NULL,
            is_lock BOOLEAN DEFAULT FALSE
        )
    ").execute(&pool).await.unwrap();

    let auth_config = Arc::new(AuthConfig {
        jwt_secret: "rat-la-bi-mat".to_string(),
    });

    // Gom nhóm các route cần bảo vệ
    let protected_routes = Router::new()
        .route("/me", get(get_me))
        .layer(middleware::from_fn_with_state(auth_config.clone(), auth_middleware));

    let app = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .nest("/api", protected_routes) // Các route bắt đầu bằng /api/me sẽ bị chặn
        .with_state(pool)
        .with_state(auth_config);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}