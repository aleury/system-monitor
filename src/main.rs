use std::{
    borrow::BorrowMut,
    error::Error,
    sync::{Arc, RwLock},
    time::Duration,
};

use askama::Template;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router, Server,
};
use sysinfo::{CpuExt, System, SystemExt};
use tokio::task::spawn_blocking;

#[derive(Clone)]
struct AppState {
    cpus: Arc<RwLock<Vec<Cpu>>>,
}

#[derive(Clone)]
struct Cpu {
    id: usize,
    usage: f32,
}

fn monitor_cpu_usage(state: AppState) {
    let mut sys = System::new();
    loop {
        sys.refresh_cpu();

        let updated_cpus: Vec<Cpu> = sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, cpu)| Cpu {
                id: i + 1,
                usage: cpu.cpu_usage(),
            })
            .collect();

        let mut cpus = state.cpus.write().unwrap();
        *cpus = updated_cpus;

        std::thread::sleep(Duration::from_millis(500));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let app_state = AppState {
        cpus: Arc::new(RwLock::new(Vec::new())),
    };

    let router = Router::new()
        .route("/", get(root_handler))
        .route("/cpu-usage", get(get_cpu_usage))
        .with_state(app_state.clone());

    spawn_blocking(move || monitor_cpu_usage(app_state));

    let addr = "0.0.0.0:3000".parse().unwrap();

    println!("Listening on {}", addr);
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    cpus: Vec<Cpu>,
}

#[derive(Template)]
#[template(path = "cpu-usage.html")]
struct CpuUsageTemplate {
    cpus: Vec<Cpu>,
}

async fn root_handler(State(state): State<AppState>) -> impl IntoResponse {
    let cpus = state.cpus.read().unwrap().clone();
    IndexTemplate { cpus }
}

async fn get_cpu_usage(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(mut socket: WebSocket, state: AppState) {
    loop {
        let cpus = state.cpus.read().unwrap().clone();
        let template = CpuUsageTemplate { cpus };
        let html = template.render().unwrap();
        socket
            .borrow_mut()
            .send(Message::Text(html))
            .await
            .unwrap_or_default();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
