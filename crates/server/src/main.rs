use anyhow::{Context, Result};
use axum::{
    routing::{get, patch, post, put},
    Router,
};
use media_elo_core::DEFAULT_TYPES;
use rusqlite::Connection;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::info;

mod csv_import;
mod db;
mod handlers;

#[derive(Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<Connection>>,
}

fn default_data_dir() -> PathBuf {
    let base = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("media-elo")
}

fn db_path() -> PathBuf {
    if let Some(p) = env::var_os("MEDIA_ELO_DB") {
        PathBuf::from(p)
    } else {
        default_data_dir().join("media.db")
    }
}

fn bind_addr() -> String {
    env::var("MEDIA_ELO_BIND").unwrap_or_else(|_| "127.0.0.1:7878".to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let db_path = db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut conn = Connection::open(&db_path)
        .with_context(|| format!("opening {}", db_path.display()))?;
    db::init(&conn)?;
    db::seed_types_if_empty(&conn, DEFAULT_TYPES)?;

    if db::count(&conn)? == 0 {
        let csv_path = db_path.with_file_name("media.csv");
        if csv_path.exists() {
            let n = csv_import::import_csv(&mut conn, &csv_path)?;
            info!("imported {n} rows from {}", csv_path.display());
        }
    }

    let state = AppState {
        conn: Arc::new(Mutex::new(conn)),
    };

    let app = Router::new()
        .route("/rows", get(handlers::list_rows).post(handlers::add_row))
        .route(
            "/rows/:id",
            put(handlers::edit_row).delete(handlers::delete_row),
        )
        .route("/rows/:id/status", patch(handlers::set_status))
        .route(
            "/types",
            get(handlers::list_types)
                .post(handlers::add_type)
                .put(handlers::reorder_types),
        )
        .route(
            "/types/:name",
            put(handlers::rename_type).delete(handlers::delete_type),
        )
        .route("/vote", post(handlers::vote))
        .route("/undo", post(handlers::undo))
        .with_state(state);

    let addr = bind_addr();
    info!("media-elo-server listening on {addr} (db: {})", db_path.display());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("shutting down");
}
