mod dirs;
mod dvdcompare;
mod error;
mod ffprobe;
mod preview;
mod rename;
mod scan;
mod state;
mod tvmaze;
mod types;

use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use state::AppState;
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "mkvlabel", about = "MKV track labeling tool")]
struct Args {
    /// Port to listen on
    #[arg(long, default_value_t = 7432)]
    port: u16,

    /// Root directory — all scan paths are relative to this
    #[arg(long, default_value = ".")]
    dir: String,
}

#[tokio::main]
async fn main() {
    // RUST_LOG controls verbosity; default to info for mkvlabel, warn for noisy deps.
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mkvlabel=debug,tower_http=debug,warn".into()),
        )
        .init();

    let args = Args::parse();

    match ffmpeg_sidecar::download::auto_download() {
        Ok(()) => info!("ffmpeg ready"),
        Err(e) => tracing::warn!("could not auto-download ffmpeg ({e}); falling back to system ffmpeg/ffprobe"),
    }

    let state = AppState::new(args.dir.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/root", get(dirs::root_handler))
        .route("/api/dirs", get(dirs::dirs_handler))
        .route("/api/scan", get(scan::handler))
        .route("/api/preview", get(preview::handler))
        .route("/api/preview/stop", get(preview::stop_handler))
        .route("/api/dvdcompare/search", get(dvdcompare::search_handler))
        .route("/api/dvdcompare/disc", get(dvdcompare::disc_handler))
        .route("/api/tvmaze/search", get(tvmaze::search_handler))
        .route("/api/tvmaze/episodes", get(tvmaze::episodes_handler))
        .route("/api/rename", post(rename::handler))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    info!("mkvlabel listening on http://{addr} (dir={})", args.dir);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
