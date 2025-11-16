use std::collections::BTreeMap;
use std::fs;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use serde::{Deserialize, Serialize};
use surge_ping::{Client, Config, PingIdentifier, PingSequence, ICMP};
use tokio::sync::Mutex;

const DOMAIN_TEST_INTERVAL: Duration = Duration::from_secs(20);
const DEVICE_PING_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Parser)]
struct Args {
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
    domains: Vec<String>,
    devices: Vec<DeviceConfig>,
}

#[derive(Clone, Serialize)]
struct DomainStatus {
    domain: String,
    status: usize,
}

#[derive(Clone, Serialize)]
struct DeviceStatus {
    name: String,
    ip: String,
    latency_milliseconds: Option<u128>,
}

#[derive(Default)]
struct HomeStatus {
    domains: BTreeMap<String, DomainStatus>,
    devices: BTreeMap<String, DeviceStatus>,
}

#[derive(Serialize)]
struct ExportHomeStatus {
    domains: Vec<DomainStatus>,
    devices: Vec<DeviceStatus>,
}

type HomeState = Arc<Mutex<HomeStatus>>;

async fn test_domain(domain: String, state: HomeState) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    loop {
        let url = format!("http://{}", domain);
        let status_code = match client.get(&url).send().await {
            Ok(response) => response.status().as_u16() as usize,
            Err(_) => 0,
        };

        let status = DomainStatus {
            domain: domain.clone(),
            status: status_code,
        };

        {
            let mut state = state.lock().await;
            state.domains.insert(domain.clone(), status);
        }

        tokio::time::sleep(DOMAIN_TEST_INTERVAL).await;
    }
}

async fn ping_device(device: DeviceConfig, state: HomeState) {
    let addr: IpAddr = device.ip.parse().unwrap();

    let kind = match addr.is_ipv6() {
        true => ICMP::V6,
        false => ICMP::V4,
    };
    let config = Config::builder().kind(kind).build();
    let client = Client::new(&config).unwrap();

    let mut pinger = client.pinger(addr, PingIdentifier(24)).await;
    pinger.timeout(Duration::from_secs(1));
    let mut ping_sequence = 0;

    loop {
        let latency_milliseconds = pinger
            .ping(PingSequence(ping_sequence), &[])
            .await
            .ok()
            .map(|(_, latency)| latency.as_millis());

        let status = DeviceStatus {
            name: device.name.clone(),
            ip: device.ip.clone(),
            latency_milliseconds,
        };

        match latency_milliseconds.is_some() {
            true => ping_sequence = ping_sequence.wrapping_add(1),
            false => ping_sequence = 0,
        }

        {
            let mut state = state.lock().await;
            state.devices.insert(device.name.clone(), status);
        }

        tokio::time::sleep(DEVICE_PING_INTERVAL).await;
    }
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../site/index.html"))
}

async fn favicon() -> Response {
    let favicon_bytes = include_bytes!("../site/favicon.svg");
    (
        [(header::CONTENT_TYPE, "image/svg+xml")],
        favicon_bytes.as_slice(),
    )
        .into_response()
}

async fn status(State(state): State<HomeState>) -> Json<ExportHomeStatus> {
    let state = state.lock().await;

    let statuses = ExportHomeStatus {
        domains: state.domains.values().cloned().collect(),
        devices: state.devices.values().cloned().collect(),
    };

    Json(statuses)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config_file = fs::read_to_string(&args.config).unwrap();
    let config: HomeConfig = serde_json::from_str(&config_file).unwrap();

    let state: HomeState = Arc::default();

    for domain in config.domains {
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            test_domain(domain, state).await;
        });
    }

    for device in config.devices {
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            ping_device(device, state).await;
        });
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/favicon.svg", get(favicon))
        .route("/status", get(status))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
