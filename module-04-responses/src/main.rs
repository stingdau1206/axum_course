use axum::{
    Json, Router,
    extract::{Path, Query},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct AuthenInf {
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Serialize)]
struct CourseInf {
    id: u64,
    title: String,
    price: f64,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: T,
}
impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize, // Bắt buộc T phải biến thành JSON được
{
    fn into_response(self) -> Response {
        // Trả về một Tuple (Status, Json)
        (StatusCode::OK, Json(self)).into_response()
    }
}

async fn pong_response() -> &'static str {
    "pong"
}

async fn check_health() -> (StatusCode, String) {
    (StatusCode::OK, format!("Server is running at {}", 2026))
}

async fn check_authen(Query(q): Query<AuthenInf>) -> Result<Redirect, (StatusCode, &'static str)> {
    if q.client_id.as_deref() == Some("koha") && q.client_secret.as_deref() == Some("180299") {
        // Nếu đúng: Trả về Redirect (Ok)
        Ok(Redirect::to("/dashboard"))
    } else {
        // Nếu sai: Trả về Tuple lỗi (Err)
        Err((StatusCode::UNAUTHORIZED, "Login failed"))
    }
}

async fn list_courses() -> ApiResponse<Vec<CourseInf>> {
    let courses = vec![CourseInf {
        id: 1,
        title: "rust".into(),
        price: 10000.0,
    }];

    // Thay vì trả về Json(courses), giờ ta trả về "vỏ bọc"
    ApiResponse {
        success: true,
        data: courses,
    }
}

async fn dashboard() -> (StatusCode, HeaderMap, Html<String>) {
    let html_content = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <title>Course Dashboard</title>
            <style>
                body { font-family: sans-serif; padding: 40px; background: #f4f4f9; }
                .card { background: white; padding: 20px; border-radius: 8px; shadow: 0 2px 5px rgba(0,0,0,0.1); }
                h1 { color: #333; }
                li { margin-bottom: 10px; color: #555; }
            </style>
        </head>
        <body>
            <div class="card">
                <h1>📊 Course Dashboard</h1>
                <p>Chào mừng bạn quay trở lại hệ thống quản lý!</p>
                <ul>
                    <li><b>Rust for Pro</b> - $99.0</li>
                    <li><b>Axum Masterclass</b> - $49.0</li>
                </ul>
                <hr>
                <a href="/api/v1/courses/json">Xem dữ liệu JSON thô</a>
            </div>
        </body>
        </html>
    "#.to_string();

    let mut headers = HeaderMap::new();
    headers.insert("X-Course-Version", HeaderValue::from_static("v1.0.0"));
    headers.insert("X-Author", HeaderValue::from_static("Koha"));

    (StatusCode::OK, headers, Html(html_content))
}

async fn get_course_detail(
    Path(id): Path<u64>,
) -> Result<ApiResponse<CourseInf>, (StatusCode, String)> {
    if id == 1 {
        Ok(ApiResponse {
            success: true,
            data: CourseInf {
                id: 1,
                title: "rust".to_string(),
                price: 10000.0,
            },
        }
    )
    } else {
        Err((StatusCode::NOT_FOUND, "Không tìm thấy khóa học".to_string()))
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/ping", get(pong_response))
        .route("/health", get(check_health))
        .route("/secret", get(check_authen))
        .route("/api/v1/courses/json", get(list_courses))
        .route("/api/v1/courses/{id}", get(get_course_detail))
        .route("/dashboard", get(dashboard));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind port 3000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
