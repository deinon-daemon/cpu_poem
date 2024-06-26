use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::Response,
    response::{Html, IntoResponse},
    routing::get,
    Router, Server,
};
use std::time ;
use rand::Rng;
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
struct Snapshot {
    cpus: Vec<f32>,
    sentences: Vec<String>
}

const CIORAN_TEXT: &str = include_str!("cioran.txt");

fn extract_sentences(cpu_usage: &[f32], significant_figures: usize) -> Vec<String> {
    // Split the text into sentences
    let sentences: Vec<&str> = CIORAN_TEXT.split('.').collect();

    // Extract a sentence for each CPU usage value
    let mut result: Vec<String> = Vec::new();
    for &usage in cpu_usage {
        // Trim the CPU usage value to the specified number of significant figures
        let trimmed_usage = trim_to_significant_figures(usage, significant_figures);

        // Convert the trimmed usage to an index
        let mut index = trimmed_usage as usize;

        // Adjust the index if it exceeds the length of sentences
        while index >= sentences.len() {
            index /= 10;
        }

        // Extract the sentence at the calculated index
        let mut sentence = sentences[index].to_string();

        if sentence.len() < 3 {
            let mut rng = rand::thread_rng();
            let random_number = rng.gen_range(1..=10);
            let new_index = index - random_number;
            sentence = sentences[new_index].to_string();
        }
        result.push(sentence);
    }

    result
}

fn trim_to_significant_figures(value: f32, significant_figures: usize) -> u32 {
    let multiplier = 10u32.pow(significant_figures as u32);
    (value * multiplier as f32).round() as u32
}


fn return_message(cpu_usage: &[f32]) -> Snapshot {
    // Calculate the average CPU usage
    let sentences = extract_sentences(cpu_usage, 4);

    // Return the selected sentence
    Snapshot { cpus: cpu_usage.to_vec(), sentences }
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<Snapshot>(1);

    tracing_subscriber::fmt::init();

    let app_state = AppState { tx: tx.clone() };

    let router = Router::new()
        .route("/", get(root_get))
        .route("/index.mjs", get(indexmjs_get))
        .route("/index.css", get(indexcss_get))
        .route("/realtime/cpus", get(realtime_cpus_get))
        .with_state(app_state.clone());

    // Update CPU usage in the background
    tokio::task::spawn_blocking(move || {
        let mut sys = System::new();
        loop {
            sys.refresh_cpu();
            let v: Vec<_> = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
            let message = return_message(&v);
            let _ = tx.send(message);
            std::thread::sleep( time::Duration::from_millis(1000)); //System::MINIMUM_CPU_UPDATE_INTERVAL);
        }
    });

    let server = Server::bind(&"0.0.0.0:7032".parse().unwrap()).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    server.await.unwrap();
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<Snapshot>,
}

#[axum::debug_handler]
async fn root_get() -> impl IntoResponse {
    let markup = tokio::fs::read_to_string("src/index.html").await.unwrap();

    Html(markup)
}

#[axum::debug_handler]
async fn indexmjs_get() -> impl IntoResponse {
    let markup = tokio::fs::read_to_string("src/index.mjs").await.unwrap();

    Response::builder()
        .header("content-type", "application/javascript;charset=utf-8")
        .body(markup)
        .unwrap()
}

#[axum::debug_handler]
async fn indexcss_get() -> impl IntoResponse {
    let markup = tokio::fs::read_to_string("src/index.css").await.unwrap();

    Response::builder()
        .header("content-type", "text/css;charset=utf-8")
        .body(markup)
        .unwrap()
}

#[axum::debug_handler]
async fn realtime_cpus_get(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|ws: WebSocket| async { realtime_cpus_stream(state, ws).await })
}

async fn realtime_cpus_stream(app_state: AppState, mut ws: WebSocket) {
    let mut rx = app_state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if let Ok(_) = ws.send(Message::Text(serde_json::to_string(&msg).unwrap())).await {
            // Message sent successfully
        } else {
            // Message sending failed
            // Handle the error or take appropriate action
            eprintln!("null message");
        }
    }
}