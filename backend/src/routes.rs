use std::{sync::Arc, time::Duration};

use axum::{
    http::HeaderValue,
    response::Response,
    routing::{get, post, put},
    Router,
};
use tower::ServiceBuilder;
use tower_governor::{governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer};
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
};

use crate::{handlers, state::AppState};

/// Everyday limit: allows normal page-load bursts of API calls without
/// being annoying, while still bounding sustained abuse. Endpoints added
/// later that need stricter limits (e.g. anything write-heavy or
/// enumeration-prone) should get their own tighter GovernorLayer, same
/// pattern as auth's login/register split.
const GENERAL_BURST: u32 = 30;
const GENERAL_REPLENISH_SECS: u64 = 1;

const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

pub fn build_router(state: AppState) -> Router {
    let origins: Vec<_> = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(AllowMethods::list([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::PATCH,
            axum::http::Method::OPTIONS,
            axum::http::Method::HEAD,
        ]))
        .allow_headers(AllowHeaders::mirror_request())
        .expose_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    let general_governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_second(GENERAL_REPLENISH_SECS)
            .burst_size(GENERAL_BURST)
            .finish()
            .expect("valid governor config"),
    );
    spawn_governor_cleanup(general_governor_config.clone());

    let api_routes = Router::new()
        .route("/health", get(handlers::health::health))
        // slide decks
        .route("/book", post(handlers::slide_decks::create_slide_deck))
        .route("/book/catalog", get(handlers::slide_decks::get_published_catalog))
        .route(
            "/book/import",
            post(handlers::slide_decks::import_slide_deck),
        )
        .route(
            "/book/owner/:owner_id",
            get(handlers::slide_decks::get_slide_decks_by_owner),
        )
        .route(
            "/book/event/:event_id",
            get(handlers::slide_decks::get_slide_decks_by_event),
        )
        .route(
            "/book/:id_or_slug/public",
            get(handlers::slide_decks::get_slide_deck_public),
        )
        .route(
            "/book/:id/export",
            get(handlers::slide_decks::export_slide_deck),
        )
        .route(
            "/book/:id",
            get(handlers::slide_decks::get_slide_deck)
                .put(handlers::slide_decks::update_slide_deck)
                .delete(handlers::slide_decks::delete_slide_deck),
        )
        // editor prefs
        .route(
            "/book/:deck_id/editor-prefs",
            get(handlers::editor_prefs::get_editor_prefs).put(handlers::editor_prefs::update_editor_prefs),
        )
        // book session
        .route(
            "/book/:deck_id/session",
            get(handlers::book_sessions::get_session).post(handlers::book_sessions::upsert_session),
        )
        .route(
            "/book/:deck_id/session/merge",
            post(handlers::book_sessions::merge_anon_into_user),
        )
        // poll votes
        .route(
            "/book/:deck_id/slides/:slide_id/elements/:element_id/poll-votes",
            get(handlers::poll_votes::get_poll_votes).post(handlers::poll_votes::submit_vote),
        )
        // author image assets
        .route(
            "/author-assets/images",
            get(handlers::author_image_assets::list)
                .post(handlers::author_image_assets::add)
                .delete(handlers::author_image_assets::remove),
        )
        .route(
            "/author-assets/images/usage",
            get(handlers::author_image_assets::usage),
        )
        // author poll templates
        .route(
            "/author-poll-templates",
            get(handlers::author_poll_templates::list).post(handlers::author_poll_templates::create),
        )
        .route(
            "/author-poll-templates/:template_id",
            put(handlers::author_poll_templates::update).delete(handlers::author_poll_templates::delete),
        )
        .route(
            "/author-poll-templates/:template_id/usage",
            get(handlers::author_poll_templates::usage),
        )
        .route(
            "/author-poll-templates/:template_id/sync",
            post(handlers::author_poll_templates::sync),
        )
        // files
        .route("/files/upload", post(handlers::files::upload))
        .route("/files/view/:file_id", get(handlers::files::view_file))
        .route(
            "/files/:file_id",
            axum::routing::delete(handlers::files::delete_file),
        )
        // Same handlers, also reachable under this domain's own path
        // prefix so the production gateway (path-based fan-out across a
        // shared /api origin, no other way to tell which backend owns a
        // given file id) can route them correctly. See eco's
        // generate_gateway_config.
        .route("/slides-files/upload", post(handlers::files::upload))
        .route("/slides-files/view/:file_id", get(handlers::files::view_file))
        .route(
            "/slides-files/:file_id",
            axum::routing::delete(handlers::files::delete_file),
        )
        .layer(GovernorLayer {
            config: general_governor_config,
        })
        .layer(
            ServiceBuilder::new()
                .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
                .layer(axum::middleware::map_response(security_headers)),
        )
        .with_state(state);

    // Mirrors the estate's other services' `server.servlet.context-path: /api`.
    Router::new().nest("/api", api_routes).layer(cors)
}

/// Same response headers as every other service in the estate.
async fn security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("x-xss-protection", HeaderValue::from_static("0"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "cache-control",
        HeaderValue::from_static("no-cache, no-store, max-age=0, must-revalidate"),
    );
    headers.insert("pragma", HeaderValue::from_static("no-cache"));
    response
}

/// The keyed rate-limit store grows one entry per distinct client key seen;
/// without periodic cleanup that's unbounded memory growth from an attacker
/// cycling source IPs.
fn spawn_governor_cleanup(
    config: Arc<tower_governor::governor::GovernorConfig<SmartIpKeyExtractor, governor::middleware::NoOpMiddleware>>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            config.limiter().retain_recent();
        }
    });
}
