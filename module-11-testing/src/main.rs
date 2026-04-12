use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    routing::get,
};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;

#[derive(Serialize, Deserialize, Clone)]
struct User {
    username: String,
    password: String,
}

type UserStore = Arc<RwLock<HashMap<u64, User>>>;

async fn health_handler() -> StatusCode {
    StatusCode::OK
}

async fn list_users(State(store): State<UserStore>) -> Json<Vec<User>> {
    let list = store.read().unwrap().values().cloned().collect();
    Json(list)
}

fn create_app(store: UserStore) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/users", get(list_users))
        .with_state(store)
}

#[cfg(test)]
mod test {
    use http_body_util::BodyExt;

    use super::*;

    fn test_store() -> UserStore {
        Arc::new(RwLock::new(HashMap::new()))
    }

    #[tokio::test]
    async fn test_health_check() {
        let app = create_app(test_store());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK)
    }

    #[tokio::test]
    async fn test_list_users() {
        let store = test_store();
        {
            let mut w = store.write().unwrap();
            w.insert(
                1,
                User {
                    username: "koha".into(),
                    password: "123".into(),
                },
            );
        }
        let app: Router = create_app(store);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/users")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // 4. Giải mã JSON từ Body để kiểm tra nội dung
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let users: Vec<User> = serde_json::from_slice(&body).unwrap();

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].username, "koha");
        // assert_eq!(response.status(), StatusCode::OK)
    }
}

#[tokio::main]
async fn main() {
    let store = Arc::new(RwLock::new(HashMap::new()));
    let app = create_app(store);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
