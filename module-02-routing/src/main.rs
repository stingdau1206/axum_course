use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{delete, get, patch, post, put},
    Router,
};
use serde::Deserialize;

// --- 1. MODELS & STRUCTS ---

#[derive(Deserialize)]
struct PostPath {
    user_id: u64,
    post_id: u64,
    comment_id: u64,
}

#[derive(Deserialize)]
struct Pagination {
    page: Option<u32>,
    limit: Option<u32>,
}

// --- 2. HANDLERS ---

// Handlers cho Resource chung (Full Actions)
async fn get_resource(Path(id): Path<u64>) -> String { format!("🔍 GET: Đọc tài nguyên {id}") }
async fn create_resource() -> &'static str { "🆕 POST: Tạo tài nguyên mới" }
async fn update_resource(Path(id): Path<u64>) -> String { format!("💾 PUT: Cập nhật toàn bộ {id}") }
async fn patch_resource(Path(id): Path<u64>) -> String { format!("🩹 PATCH: Cập nhật một phần {id}") }
async fn delete_resource(Path(id): Path<u64>) -> String { format!("🗑️ DELETE: Xóa tài nguyên {id}") }

// Handlers cho User/Post/Comment
async fn list_users() -> &'static str { "👥 Danh sách người dùng" }
async fn get_user(Path(id): Path<u64>) -> String { format!("👤 Chi tiết User: {id}") }
async fn get_post(Path((u, p)): Path<(u64, u64)>) -> String { format!("📝 User {u} - Post {p}") }
async fn get_comment(Path(params): Path<PostPath>) -> String {
    format!("💬 User {} -> Post {} -> Comment {}", params.user_id, params.post_id, params.comment_id)
}

async fn list_items(Query(q): Query<Pagination>) -> String {
    format!("📦 Items - Page: {}, Limit: {}", q.page.unwrap_or(1), q.limit.unwrap_or(10))
}

async fn files(Path(path): Path<String>) -> String { format!("📂 File: {path}") }

// --- 3. MODULAR ROUTERS ---

// QUAN TRỌNG: Resource Route với Full Actions
fn resource_routes() -> Router {
    Router::new()
        // Tại địa chỉ /resource/
        .route("/", post(create_resource))
        // Tại địa chỉ /resource/{id} - Gộp tất cả các method vào đây
        .route(
            "/{id}",
            get(get_resource)
                .put(update_resource)
                .patch(patch_resource)
                .delete(delete_resource),
        )
}

fn comment_routes() -> Router {
    Router::new().route("/{comment_id}", get(get_comment))
}

fn post_routes() -> Router {
    Router::new()
        .route("/{post_id}", get(get_post))
        .nest("/{post_id}/comments", comment_routes())
}

fn user_routes() -> Router {
    Router::new()
        .route("/", get(list_users))
        .route("/{user_id}", get(get_user))
        .nest("/{user_id}/posts", post_routes())
}

// --- 4. MAIN APP ---

#[tokio::main]
async fn main() {
    let api_v1 = Router::new()
        .nest("/users", user_routes())
        .nest("/resource", resource_routes()) // Thêm resource routes vào đây
        .route("/items", get(list_items));

    let app = Router::new()
        .route("/", get(|| async { "🚀 Axum 0.8 Full Routing Module Ready!" }))
        .nest("/api/v1", api_v1)
        .route("/static/{*path}", get(files))
        .fallback(|| async { (StatusCode::NOT_FOUND, "🚫 404 Not Found") });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    
    println!("\n--- 🛠  DANH SÁCH ENDPOINT ---");
    println!("✅ FULL CRUD:  GET/PUT/PATCH/DELETE  http://localhost:3000/api/v1/resource/123");
    println!("✅ NESTED:     GET                   http://localhost:3000/api/v1/users/1/posts/2/comments/3");
    println!("✅ WILDCARD:   GET                   http://localhost:3000/static/any/path/file.txt");
    println!("----------------------------\n");

    axum::serve(listener, app).await.unwrap();
}