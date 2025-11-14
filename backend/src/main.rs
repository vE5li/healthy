use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use serde::{Deserialize, Serialize};
use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

type DeviceState = Arc<Mutex<HashMap<String, bool>>>;

#[derive(Parser)]
struct Args {
    /// Path to the devices configuration file
    #[arg(short, long, default_value = "devices.json")]
    config: String,
    #[arg(short, long, default_value = "4901")]
    port: u16,
}

#[derive(Deserialize)]
struct DeviceConfig {
    devices: Vec<String>,
}

#[derive(Serialize)]
struct DeviceStatus {
    ip: String,
    connected: bool,
}

async fn ping_device(ip: String, state: DeviceState) {
    let client = Client::new(&Config::default()).unwrap();

    let addr = ip.parse().unwrap();
    let mut pinger = client.pinger(addr, PingIdentifier(24)).await;
    pinger.timeout(Duration::from_secs(1));

    loop {
        let connected = pinger.ping(PingSequence(0), &[]).await.is_ok();

        {
            let mut map = state.lock().await;
            map.insert(ip.clone(), connected);
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn get_status(State(state): State<DeviceState>) -> Json<Vec<DeviceStatus>> {
    let map = state.lock().await;
    let statuses = map
        .iter()
        .map(|(ip, connected)| DeviceStatus {
            ip: ip.clone(),
            connected: *connected,
        })
        .collect();
    Json(statuses)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config_file = fs::read_to_string(&args.config).unwrap();
    let config: DeviceConfig = serde_json::from_str(&config_file).unwrap();

    let state: DeviceState = Arc::new(Mutex::new(HashMap::new()));

    for device in config.devices {
        let device_state = Arc::clone(&state);
        tokio::spawn(async move {
            ping_device(device, device_state).await;
        });
    }

    let app = Router::new()
        .route("/status", get(get_status))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", args.port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
