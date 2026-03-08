use std::sync::{Arc, RwLock};

use axum::{
    Json, Router,
    extract::{FromRef, State},
    http::StatusCode,
    routing::get,
    Extension
};
use serde::{Deserialize, Serialize};

type Inventory = Arc<RwLock<Vec<Product>>>;

#[derive(Serialize, Clone)]
struct StoreConfig {
    name: String,
    location: String,
    opening_year: u16,
}

#[derive(Deserialize, Serialize, Clone)]
struct Product {
    id: u32,
    name: String,
    quantity: u32,
}

#[derive(Clone)]
struct AppState {
    config: Arc<StoreConfig>,
    db: Database,
}

impl FromRef<AppState> for Arc<StoreConfig> {
    fn from_ref(input: &AppState) -> Self {
        input.config.clone()
    }
}

impl FromRef<AppState> for Database {
    fn from_ref(input: &AppState) -> Self {
        input.db.clone()
    }
}

#[derive(Clone)]
struct Database {
    inventory: Inventory,
}

impl Database {
    fn new() -> Self {
        Self {
            inventory: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn add_items(&self, product: Product) {
        let mut list = self.inventory.write().unwrap();
        list.push(product);
    }

    fn get_all_items(&self) -> Vec<Product> {
        let list = self.inventory.read().unwrap();
        list.clone()
    }
}

#[derive(Clone)]
struct ManagerInfo {
    name: String
}

async fn store_info(State(config): State<Arc<StoreConfig>>) -> Json<StoreConfig> {
    Json((*config).clone())
}

async fn add_product(State(db): State<Database>, Extension(manager): Extension<ManagerInfo>,Json(payload): Json<Product>) -> (StatusCode, String) {
    db.add_items(payload);
    (StatusCode::CREATED, format!("Created by {}", manager.name))
}

async fn list_product(State(db): State<Database>) -> Json<Vec<Product>> {
    Json(db.get_all_items())
}

#[tokio::main]
async fn main() {
    let state = AppState {
        config: Arc::new(StoreConfig {
            name: "koha".to_string(),
            location: "CR".to_string(),
            opening_year: 2026,
        }),
        db: Database::new()
    };
    let manager = ManagerInfo { name: "Koha Admin".to_string() };

    let app = Router::new()
        .route("/info", get(store_info))
        .route("/products", get(list_product).post(add_product))
        .layer(Extension(manager))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind port 3000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
