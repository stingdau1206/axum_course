use std::sync::{Arc, RwLock};

use axum::{
    Router,
    extract::{Path, Query, Request, State},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response}, routing::post,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

type ArcBooks = Arc<RwLock<Vec<Book>>>;

#[derive(Clone, Serialize, Deserialize)]
struct Book {
    id: u64,
    title: String,
    author: String,
    is_available: bool,
}

#[derive(Error, Debug)]
enum LibraryError {
    #[error("Sách với id {0} không tồn tại")]
    BookNotFound(u64),

    #[error("{0} không được trống")]
    InvalidInput(String),

    #[error("Lỗi xác thực")]
    Unauthorized,

    #[error("Sách đã được mượn")]
    AlreadyBorrowed,
}

impl IntoResponse for LibraryError {
    fn into_response(self) -> Response {
        match self {
            LibraryError::AlreadyBorrowed => (StatusCode::CONFLICT, self.to_string()).into_response(),
            LibraryError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
            LibraryError::BookNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
            LibraryError::InvalidInput(_) => (StatusCode::CONFLICT, self.to_string()).into_response()
        }
    }
}

async fn logging_middleware(request: Request, next: Next) -> Response {
    next.run(request).await
}

async  fn authen_middleware(request: Request, next: Next) -> Result<Response, LibraryError> {
    let authen = request.headers().get("X-Admin-Token").and_then(|token| token.to_str().ok());
    match authen {
        Some("library-admin") => Ok(next.run(request).await),
        _ => Err(LibraryError::Unauthorized)
    }
}

async fn list_books(State(books): State<ArcBooks>) -> Vec<Book> {
    books.read().unwrap().clone()
}

async fn add_book(State(books): State<ArcBooks>, Query(book_inf): Query<Book>) -> Result<String, LibraryError>{
    if book_inf.title.is_empty() {
        return Err(LibraryError::InvalidInput("title".to_string()));
    }
    books.write().unwrap().push(book_inf);
    Ok(format!("add book done"))
} 

async fn borrow_book(State(books): State<ArcBooks>, Path(id): Path<u64>) -> Result<Book, LibraryError> {
    let book =     books.read().unwrap().clone()[0].clone();
    if book.is_available {
        return Ok(book);
    }
    Err(LibraryError::AlreadyBorrowed)
}


#[tokio::main]
async fn main() {
    let books: ArcBooks = Arc::new(RwLock::new(Vec::new()));

    let route = Router::new()
        .route("/books", post(add_book))
        .layer(middleware::from_fn(authen_middleware));

    let app = Router::new()
        .nest("/", route)
        .route("/books", get(list_books))
        .route("/books/{id}/borrow", post(borrow_book))
        .with_state(books);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind port 3000");
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
