use std::env;

use argon2::{Argon2, password_hash::SaltString};
use axum::{
    Extension, Json, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{Duration, Utc};
use dotenvy::dotenv;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Clone)]
struct CurrentUser {
    user_id: uuid::Uuid,
}

#[derive(Debug, thiserror::Error)]
enum DbError {
    #[error("User not found")]
    NotFound,
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for DbError {
    fn into_response(self) -> Response {
        let(status, msg) = match self {
            DbError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            DbError::Sqlx(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
        };

        (status, msg).into_response()
    }
}

#[derive(Deserialize, Serialize)]
struct Claims {
    sub: String,
    exp: usize,
}

#[derive(Clone)]
struct AppState {
    secret_key: String,
    db: PgPool,
}
#[derive(Deserialize)]
struct LoginPayload {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
}

#[derive(sqlx::FromRow)]
struct User {
    id: uuid::Uuid,
    username: String,
    password: String,
}

async fn register_handler(State(state): State<AppState>, Json(body): Json<LoginPayload>) -> Result<StatusCode, DbError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    sqlx::query(
        "INSERT INTO users (id, username, password) VALUES ($1, $2, $3)"
    )
    .bind(uuid::Uuid::new_v4())
    .bind(body.username)
    .bind(body.password)
    .execute(&state.db)
    .await?;

    Ok(StatusCode::CREATED)
}

async fn login_handler(
    State(state): State<AppState>,
    Json(body): Json<LoginPayload>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&body.username)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if user.password != body.password {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let exp = Utc::now()
        .checked_add_signed(Duration::hours(1))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.id.to_string(),
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.secret_key.as_ref()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AuthResponse { token }))
}

async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let token_data = match auth_header {
        Some(token) => {
            let secret = state.secret_key;
            decode::<Claims>(
                token,
                &DecodingKey::from_secret(secret.as_ref()),
                &Validation::new(Algorithm::HS256),
            )
            .map_err(|_| StatusCode::UNAUTHORIZED)?
        }
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    let user_uuid =
        uuid::Uuid::parse_str(&token_data.claims.sub).map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut()
        .insert(CurrentUser { user_id: user_uuid });
    Ok(next.run(req).await)
}

async fn secret_dashboard(Extension(user): Extension<CurrentUser>) -> String {
    format!(
        "Chào sếp có ID là: {}. Đây là dữ liệu tối mật!",
        user.user_id
    )
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let secret_key = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env");
    let database_url = env::var("DATABASE_URL").expect("JWT_SECRET must be set in .env");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("cant connect to database");

    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS users (id UUID PRIMARY KEY,
        username TEXT NOT NULL UNIQUE,
        password TEXT NOT NULL)",
    )
    .execute(&pool)
    .await
    .expect("can't create table users");

    let state = AppState {
        secret_key: secret_key,
        db: pool,
    };

    let admin_routes = Router::new()
        .route("/dashboard", get(secret_dashboard))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let app = Router::new()
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .nest("/admin", admin_routes)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind port 3000");
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
