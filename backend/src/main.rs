mod auth_extractor;
mod config;
mod error;
mod handlers;
mod jwt;
mod routes;
mod state;

use config::AppConfig;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = AppConfig::from_env()?;

    let client = mongodb::Client::with_uri_str(&config.mongodb_uri).await?;
    let db = client
        .default_database()
        .ok_or_else(|| anyhow::anyhow!("MONGODB_URI must include a database name"))?;

    tracing::info!(port = config.server_port, "starting slides-service");

    let state = AppState::new(db, config.clone());
    let app = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.server_port)).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
