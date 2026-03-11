use std::env;

use axum::{Json, Router, extract::{Path, State}, http::StatusCode, response::{IntoResponse, Response}, routing::{get, post}};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Pool, Postgres, postgres::PgPoolOptions};
use uuid::Uuid;

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
            DbError::NotFound => (StatusCode::NOT_FOUND, "Book not fount"),
            DbError::Sqlx(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        };

        (status, msg).into_response()
    }
}

#[derive(sqlx::FromRow, Deserialize, Serialize)]
struct Book {
    id: uuid::Uuid,
    title: String,
    author: String,
    is_available: bool,
}

#[derive(Deserialize)]
struct CreateBook {
    title: String,
    author: String
}

#[derive(Deserialize, Serialize)]
struct UpdateBook {
    title: Option<String>,
    author: Option<String>,
    is_available: Option<bool>,
}

async fn create_book(State(pool) : State<Pool<Postgres>>, Json(book): Json<CreateBook>) -> Result<(StatusCode, Json<Book>), DbError> {

    let new_book = sqlx::query_as::<_, Book>(
        "INSERT INTO books (id, title, author, is_available) VALUES ($1, $2, $3, $4) RETURNING *"
    )
    .bind(Uuid::new_v4()) // Sinh ID mới ở đây
    .bind(&book.title) // title
    .bind(&book.author) // author
    .bind(true) // mặc định là có sẵn
    .fetch_one(&pool) // fetch_one vì ta chắc chắn nó sẽ trả về 1 dòng (nhờ RETURNING *)
    .await?; // nếu failed sẽ trả 

    Ok((StatusCode::CREATED, Json(new_book)))
}

async fn list_books(State(pool) : State<PgPool>) -> Result<Json<Vec<Book>>, DbError> {
    let books = sqlx::query_as::<_, Book>("SELECT * FROM books")
        .fetch_all(&pool)
        .await?;
    Ok(Json(books))
}

async fn get_book(State(pool) : State<PgPool>, Path(id) : Path<uuid::Uuid>) -> Result<Json<Book>, DbError> {
    let book = sqlx::query_as::<_, Book>("SELECT * FROM books WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .ok_or(DbError::NotFound)?;
    Ok(Json(book))
}

async fn delete_book(State(pool) : State<PgPool>, Path(id) : Path<uuid::Uuid>) -> Result<StatusCode, DbError> {
    let result = sqlx::query("DELETE FROM books WHERE id = $1")
            .bind(id)
            .execute(&pool) // Dùng execute để thực thi lệnh xóa
            .await?;

    if result.rows_affected() == 0 {
        Err(DbError::NotFound)
    }
    else {
        Ok(StatusCode::NO_CONTENT)
    }
}

async fn update_book(State(pool) : State<Pool<Postgres>>, Path(id): Path<uuid::Uuid>, Json(input): Json<UpdateBook>) -> Result<Json<Book>, DbError> {
    let book = sqlx::query_as::<_, Book>(
            "UPDATE books 
            SET title = COALESCE($2, title), 
                author = COALESCE($3, author),
                is_available = COALESCE($4, is_available)
            WHERE id = $1 
            RETURNING *"
        )
        .bind(id)           // $1
    .bind(&input.title)  // $2
    .bind(&input.author) // $3
    .bind(input.is_available) // $4
    .fetch_optional(&pool)
    .await?             // Bước 1: Check lỗi DB
    .ok_or(DbError::NotFound)?; // Bước 2: Check xem ID có tồn tại để update không

    Ok(Json(book))
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Can't connect to Database");

    sqlx::query("
        CREATE TABLE IF NOT EXISTS books (id UUID PRIMARY KEY,
        title TEXT NOT NULL,
        author TEXT NOT NULL,
        is_available BOOLEAN)")
    .execute(&pool)
    .await
    .expect("can't create table books");

    let app = Router::new()
        .route("/books", post(create_book).get(list_books))
        .route("/books/{id}", get(get_book).delete(delete_book).put(update_book).patch(update_book))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.expect("Failed to start server");
}
