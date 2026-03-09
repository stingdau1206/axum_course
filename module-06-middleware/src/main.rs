use std::sync::{Arc, RwLock};
use axum::{
    Router,
    extract::{Path, Request, State, Json},
    http::{StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response}, 
    routing::{get, post}, // Thêm get vào đây
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

type ArcBooks = Arc<RwLock<Vec<Book>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[error("Lỗi xác thực: Token không hợp lệ")]
    Unauthorized,
    #[error("Sách đã được mượn trước đó")]
    AlreadyBorrowed,
}

impl IntoResponse for LibraryError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            LibraryError::BookNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            LibraryError::InvalidInput(_) => (StatusCode::BAD_REQUEST, self.to_string()), // Đổi thành 400
            LibraryError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            LibraryError::AlreadyBorrowed => (StatusCode::CONFLICT, self.to_string()),
        };

        let body = Json(serde_json::json!({
            "error": msg,
            "code": status.as_u16()
        }));

        (status, body).into_response()
    }
}

// 1. Middleware ghi log đơn giản
async fn logging_middleware(request: Request, next: Next) -> Response {
    println!("Request: {} {}", request.method(), request.uri());
    next.run(request).await
}

// 2. Middleware xác thực
async fn authen_middleware(request: Request, next: Next) -> Result<Response, LibraryError> {
    let auth_header = request.headers()
        .get("X-Admin-Token")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some("library-admin") => Ok(next.run(request).await),
        _ => Err(LibraryError::Unauthorized),
    }
}

// --- Handlers ---

async fn list_books(State(books): State<ArcBooks>) -> Json<Vec<Book>> {
    let b = books.read().unwrap();
    Json(b.clone())
}

async fn add_book(
    State(books): State<ArcBooks>, 
    Json(new_book): Json<Book> // Dùng Json thay vì Query
) -> Result<(StatusCode, String), LibraryError> {
    if new_book.title.trim().is_empty() {
        return Err(LibraryError::InvalidInput("title".to_string()));
    }
    
    let mut b = books.write().unwrap();
    b.push(new_book);
    Ok((StatusCode::CREATED, "Thêm sách thành công".to_string()))
} 

async fn borrow_book(
    State(books): State<ArcBooks>, 
    Path(id): Path<u64>
) -> Result<Json<Book>, LibraryError> {
    let mut b = books.write().unwrap();
    
    // Tìm sách theo ID
    let book = b.iter_mut()
        .find(|book| book.id == id)
        .ok_or(LibraryError::BookNotFound(id))?;

    if !book.is_available {
        return Err(LibraryError::AlreadyBorrowed);
    }

    book.is_available = false; // Cập nhật trạng thái
    Ok(Json(book.clone()))
}

#[tokio::main]
async fn main() {
    // Khởi tạo State với một vài cuốn sách mẫu
    let initial_books = vec![
        Book { id: 1, title: "Rust in Action".into(), author: "Tim".into(), is_available: true },
    ];
    let books: ArcBooks = Arc::new(RwLock::new(initial_books));

    // Nhóm các Route cần Admin (Middleware bảo vệ)
    let admin_routes = Router::new()
        .route("/", post(add_book))
        .layer(middleware::from_fn(authen_middleware));

    // Router tổng hợp
    let app = Router::new()
        .nest("/admin/books", admin_routes) // Đường dẫn admin
        .route("/books", get(list_books))    // Công khai
        .route("/books/{id}/borrow", post(borrow_book)) // Công khai
        .layer(middleware::from_fn(logging_middleware)) // Log cho mọi request
        .with_state(books);

    println!("🚀 Server running at http://0.0.0.0:3000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}