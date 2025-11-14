use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use serde::{Deserialize, Serialize};
use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tokio::sync::Mutex;

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
    name: String,
    ip: String,
}

#[derive(Deserialize)]
struct HomeConfig {
    devices: Vec<DeviceConfig>,
}

#[derive(Clone, Serialize)]
struct DeviceStatus {
    name: String,
    ip: String,
    latency_milliseconds: Option<u128>,
}

type DeviceMap = Arc<Mutex<HashMap<String, DeviceStatus>>>;

async fn ping_device(device: &DeviceConfig, state: DeviceMap) {
    let client = Client::new(&Config::default()).unwrap();

    let addr = device.ip.parse().unwrap();
    let mut pinger = client.pinger(addr, PingIdentifier(24)).await;
    pinger.timeout(Duration::from_secs(1));

    loop {
        let latency_milliseconds = pinger.ping(PingSequence(0), &[]).await.ok().map(|(_, latency)| latency.as_millis());
        let status = DeviceStatus {
            name: device.name.clone(),
            ip: device.ip.clone(),
            latency_milliseconds,
        };

        {
            let mut map = state.lock().await;
            map.insert(device.name.clone(), status);
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn get_status(State(state): State<DeviceMap>) -> Json<Vec<DeviceStatus>> {
    let map = state.lock().await;
    let statuses = map.values().cloned().collect();
    Json(statuses)
}

async fn index() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Device Status</title>
    <style>
        body {
            font-family: sans-serif;
            padding: 2rem;
            max-width: 800px;
            margin: 0 auto;
        }
        h1 {
            color: #333;
        }
        ul {
            list-style: none;
            padding: 0;
        }
        li {
            margin: 0.5rem 0;
            padding: 0.5rem;
            border-radius: 4px;
            background: #f5f5f5;
        }
        .connected {
            color: green;
        }
        .disconnected {
            color: red;
        }
    </style>
</head>
<body>
    <h1>Device Status</h1>
    <div id="status">Loading...</div>

    <script>
        async function checkStatus() {
            try {
                const res = await fetch('/status');
                const devices = await res.json();

                if (devices.length === 0) {
                    document.getElementById('status').innerHTML = '<p>Loading...</p>';
                } else {
                    const html = '<ul>' + devices.map(device => {
                        const className = device.latency_milliseconds ? 'connected' : 'disconnected';
                        const icon = device.latency_milliseconds ? '✓' : '✗';
                        const latency = device.latency_milliseconds ? `(${device.latency_milliseconds} ms)` : '';

                        return `<li><span class="${className}">${icon} ${device.ip} ${latency}</span></li>`;
                    }).join('') + '</ul>';
                    document.getElementById('status').innerHTML = html;
                }
            } catch (err) {
                document.getElementById('status').innerHTML = '<p>Error loading status</p>';
            }
        }

        checkStatus();
        setInterval(checkStatus, 500);
    </script>
</body>
</html>"#,
    )
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config_file = fs::read_to_string(&args.config).unwrap();
    let config: HomeConfig = serde_json::from_str(&config_file).unwrap();

    let state: DeviceMap = Arc::new(Mutex::new(HashMap::new()));

    for device in config.devices {
        let device_state = Arc::clone(&state);
        tokio::spawn(async move {
            ping_device(&device, device_state).await;
        });
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/status", get(get_status))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", args.port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
