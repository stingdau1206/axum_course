use std::{convert::Infallible, time::Duration};

use axum::{Router, extract::{Multipart, WebSocketUpgrade, multipart::MultipartError, ws::{Message, WebSocket}}, http::StatusCode, response::{IntoResponse, Sse, sse::{Event, KeepAlive}}, routing::{get, post}};
use tokio_stream::StreamExt; // Để dùng hàm .throttle()
use futures::stream::{self, Stream};
use tower_http::services::ServeDir; // Đây là chỗ chứa repeat_with

async fn handle_socket(mut socket: WebSocket) {
    socket.send(Message::Text("Hello World".into())).await.ok();
    while let Some(msg) = socket.recv().await {
        if let Ok(Message::Text(text)) = msg {
            let response = format!("Echo: {}", text);
            socket.send(Message::Text(response.into())).await.ok();
        }
    }   
    println!("disconected");     
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn sse_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::repeat_with(|| {
        Event::default().data(format!("Server time: {:?}", std::time::SystemTime::now()))
    })
    .map(Ok)
    .throttle(Duration::from_secs(1));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn upload(mut multipart: Multipart) -> Result<String, MultipartError> {
    let mut files = Vec::new();

    // Vòng lặp lấy từng "ngăn" (field) trong container
    while let Some(field) = multipart.next_field().await? {
        // Lấy tên của trường dữ liệu (ví dụ: "avatar", "document")
        let name = field.name().unwrap_or("unknown").to_string();
        
        // Đọc toàn bộ nội dung file dưới dạng mảng byte
        let data = field.bytes().await?;
        
        files.push(format!("{}: {} bytes", name, data.len()));
    }

    // Trả về thông báo cho Client
    if files.is_empty() {
        Ok("No files uploaded".to_string())
    } else {
        Ok(format!("Uploaded: {}", files.join(", ")))
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
    .route("/ws", get(ws_handler))
    .route("/sse", get(sse_handler))
    .route("/upload", post(upload))
    .nest_service("/static", ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.expect("failed to bind port 3000");
    axum::serve(listener, app).await.expect("Failed to start server");
}
