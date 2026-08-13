use axum::{extract::Path, routing::get, Router};
use tower_http::services::ServeDir;

// ecobook frontend: serves the Phaser portrait reader page.
// The reader fetches deck data from the estate gateway's /api/book/* route
// (routed to the ecobook backend LXS), so no API proxying is needed here.

async fn reader(Path(slug): Path<String>) -> axum::response::Html<String> {
    let html = std::fs::read_to_string("static/reader.html").unwrap_or_default();
    axum::response::Html(html.replace("__SLUG__", &slug))
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .or_else(|| std::env::var("PORT").ok().and_then(|p| p.parse().ok()))
        .unwrap_or(8280);
    let app = Router::new()
        .route("/ecobook/:slug", get(reader))
        .route("/", get(|| async { axum::response::Html("ecobook reader") }))
        .nest_service("/static", ServeDir::new("static"));
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    println!("[ecobook-frontend] listening on :{port}");
    axum::serve(listener, app).await.unwrap();
}
