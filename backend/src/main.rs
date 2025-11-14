use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::process::Command;
use tower_http::cors::CorsLayer;

#[derive(Serialize)]
struct Status {
    connected: bool,
}

async fn check_status() -> Json<Status> {
    let output = Command::new("ping")
        .args(["-c", "1", "-W", "1", "8.8.8.8"])
        .output();

    let connected = output.map(|o| o.status.success()).unwrap_or(false);
    Json(Status { connected })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/status", get(check_status))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:4901")
        .await
        .unwrap();

    println!("Backend running on http://127.0.0.1:4901");

    axum::serve(listener, app).await.unwrap();
}
