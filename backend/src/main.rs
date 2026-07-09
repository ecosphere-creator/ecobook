use slides_service::{config::AppConfig, routes, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    // JSON, not the default human-readable text: every field (including
    // the request_id set by request_id::propagate on the request's span)
    // ends up as a real, queryable field instead of buried in a formatted
    // string -- what a log aggregator like Loki actually wants.
    tracing_subscriber::fmt()
        .json()
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
