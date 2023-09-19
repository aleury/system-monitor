use std::{error::Error, sync::Arc};

use askama::Template;
use axum::{extract::State, response::IntoResponse, routing::get, Router, Server};
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::RwLock;

type SysInfo = Arc<RwLock<System>>;

#[derive(Clone)]
struct AppState {
    sys: SysInfo,
}

impl AppState {
    async fn get_cpu_usage(&self) -> Vec<Cpu> {
        let mut sys = self.sys.write().await;
        sys.refresh_cpu();
        let cpus: Vec<Cpu> = sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, cpu)| Cpu {
                id: i + 1,
                usage: cpu.cpu_usage(),
            })
            .collect();
        cpus
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut sys = System::new();
    sys.refresh_cpu();

    let app_state = AppState {
        sys: Arc::new(RwLock::new(sys)),
    };

    let router = Router::new()
        .route("/", get(root_handler))
        .route("/cpu-usage", get(get_cpu_usage))
        .with_state(app_state);

    let addr = "0.0.0.0:3000".parse().unwrap();

    println!("Listening on {}", addr);
    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

struct Cpu {
    id: usize,
    usage: f32,
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
    let cpus = state.get_cpu_usage().await;
    IndexTemplate { cpus }
}

async fn get_cpu_usage(State(state): State<AppState>) -> impl IntoResponse {
    let cpus = state.get_cpu_usage().await;
    CpuUsageTemplate { cpus }
}
