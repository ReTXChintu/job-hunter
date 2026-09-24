use std::net::SocketAddr;
use std::time::Duration;

use job_hunter_relay::{build_router, AppState, Db, RelayConfig};

#[tokio::main]
async fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_env("JOB_HUNTER_RELAY_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,job_hunter_relay=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let config = RelayConfig::from_env();
    tracing::info!(bind = %config.bind_addr, db = %config.database_path.display(), "starting job-hunter-relay");
    let db = Db::open(&config.database_path).expect("failed to open relay database");
    let bind_addr = config.bind_addr.clone();
    let state = AppState::new(config, db);

    {
        let db = state.db.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                if let Err(e) = db.cleanup_expired(chrono::Utc::now()) {
                    tracing::warn!(error = %e, "housekeeping pass failed");
                }
            }
        });
    }

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {bind_addr}: {e}"));
    tracing::info!(addr = %bind_addr, "job-hunter-relay listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("server error");
}
