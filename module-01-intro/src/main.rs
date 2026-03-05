use axum::{
    routing::{get, post},
    Router,
};

async fn hello_world() -> &'static str{
    "Hello World"
}
use axum::http::StatusCode;

async fn conditional_response() -> (StatusCode, &'static str) {
    let is_working = true;
    
    if is_working {
        (StatusCode::OK, "Everything is working!")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Service is down")
    }
}

async fn echo(body: String) -> String {
    format!("You sent: {}", body)
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/status", get(conditional_response))
        .route("/echo", post(echo));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.expect("Failed to bind port 3000");
    
    axum::serve(listener, app).await.expect("Server failed to start");  
}
