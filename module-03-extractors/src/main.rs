use axum::{
    extract::{FromRequest, FromRequestParts, Path, Query, State},
    http::{request::Parts, Request, StatusCode},
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ============================================================================
// 1. MODELS & STATE
// ============================================================================

struct AppState {
    db_pool: String,
    api_version: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct CreateCourseRequest {
    title: String,
    description: String,
}

#[derive(Serialize)]
struct CourseResponse {
    id: u64,
    title: String,
    description: String,
}

#[derive(Deserialize)]
struct Pagination {
    page: Option<u32>,
    limit: Option<u32>,
}

// ============================================================================
// 2. CUSTOM EXTRACTORS
// ============================================================================

// --- EXTRACTOR 1: AdminToken (Kiểm tra Header - FromRequestParts) ---
struct AdminToken;

impl<S> FromRequestParts<S> for AdminToken
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("X-Auth-Token")
            .and_then(|v| v.to_str().ok());

        if let Some("secret-admin") = token {
            Ok(AdminToken)
        } else {
            Err((StatusCode::UNAUTHORIZED, "🚫 Lỗi: Bạn cần quyền Admin!"))
        }
    }
}

// --- EXTRACTOR 2: ValidatedCourse (Kiểm tra Body - FromRequest) ---
struct ValidatedCourse(CreateCourseRequest);

impl<S> FromRequest<S> for ValidatedCourse
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        // Sử dụng extractor Json mặc định của Axum để lấy dữ liệu trước
        let Json(payload) = Json::<CreateCourseRequest>::from_request(req, state)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Lỗi JSON: {}", e)))?;

        // Thực hiện logic kiểm tra (Validation)
        if payload.title.trim().is_empty() {
            return Err((StatusCode::BAD_REQUEST, "❌ Tiêu đề không được để trống!".to_string()));
        }
        if payload.title.len() < 3 {
            return Err((StatusCode::BAD_REQUEST, "❌ Tiêu đề phải có ít nhất 3 ký tự!".to_string()));
        }

        Ok(ValidatedCourse(payload))
    }
}

// ============================================================================
// 3. HANDLERS
// ============================================================================

async fn list_courses(Query(q): Query<Pagination>) -> String {
    format!(
        "📚 Danh sách khóa học - Trang: {}, Giới hạn: {}",
        q.page.unwrap_or(1),
        q.limit.unwrap_or(10)
    )
}

async fn get_course(Path(id): Path<u64>) -> String {
    format!("📖 Thông tin khóa học ID: {id}")
}

// Sử dụng ValidatedCourse thay vì Json trực tiếp
async fn create_course(ValidatedCourse(payload): ValidatedCourse) -> Json<CourseResponse> {
    Json(CourseResponse {
        id: 101, // Giả định ID tạo mới
        title: payload.title,
        description: payload.description,
    })
}

async fn delete_course(_admin: AdminToken, Path(id): Path<u64>) -> String {
    format!("🗑️ Admin đã xóa thành công khóa học ID: {id}")
}

async fn check_state(State(state): State<Arc<AppState>>) -> String {
    format!("⚙️ System: {}, DB: {}", state.api_version, state.db_pool)
}

// ============================================================================
// 4. ROUTING & MAIN
// ============================================================================

fn courses_routes() -> Router {
    Router::new()
        .route("/", get(list_courses).post(create_course))
        .route("/{id}", get(get_course).delete(delete_course))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        db_pool: "postgres://localhost:5432/courses".to_string(),
        api_version: "v1.0-stable".to_string(),
    });

    let app = Router::new()
        .route("/api/v1/system/status", get(check_state))
        .with_state(state)
        .nest("/api/v1/courses", courses_routes())
        .fallback(|| async { (StatusCode::NOT_FOUND, "📍 Trang này không tồn tại!") });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    
    println!("\n🚀 COURSE MANAGEMENT SYSTEM IS ONLINE");
    println!("1. List Courses:   GET  http://localhost:3000/api/v1/courses");
    println!("2. Create Course: POST http://localhost:3000/api/v1/courses (Body JSON)");
    println!("3. Delete Course: DELETE http://localhost:3000/api/v1/courses/1 (Header X-Auth-Token)");
    println!("------------------------------------------------------------\n");

    axum::serve(listener, app).await.unwrap();
}