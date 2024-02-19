use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use vulture::{DefaultScanner, MemoryScanner, ProcessHandle, ScanType};

#[derive(Default)]
struct AState {
    process: Option<ProcessHandle>,
    scanner: Option<DefaultScanner>,
}

type ServerState = Arc<Mutex<AState>>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let port = std::env::var("PORT")
        .unwrap_or("3000".to_string())
        .parse::<u32>()?;

    let router = Router::new()
        .route("/attach", get(attach))
        .route("/scan", get(scan))
        .with_state(Arc::new(Mutex::new(AState::default())));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

#[derive(Deserialize)]
struct Attach {
    pid: u32,
}

async fn attach(Query(Attach { pid }): Query<Attach>, State(state): State<ServerState>) {
    let proc = ProcessHandle::new(pid as _);
    let scanner = DefaultScanner::new(proc.clone());

    let mut state = state.lock().await;
    state.process = Some(proc);
    state.scanner = Some(scanner);
}

#[derive(Deserialize)]
struct ScanRequest {}

#[derive(Serialize, Default)]
struct ScanResponse {
    addresses: Vec<usize>,
}

async fn scan(
    Query(_scan): Query<ScanRequest>,
    State(state): State<ServerState>,
) -> (StatusCode, Json<ScanResponse>) {
    let mut state = state.lock().await;
    let Some(ref mut scanner) = state.scanner else {
        return (StatusCode::NO_CONTENT, Json(Default::default()));
    };
    let Ok(results) = scanner.find_values(&4, |a, b| a == b, ScanType::Initialize) else {
        return (StatusCode::NO_CONTENT, Json(Default::default()));
    };

    (
        StatusCode::OK,
        Json(ScanResponse {
            addresses: results.clone(),
        }),
    )
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
