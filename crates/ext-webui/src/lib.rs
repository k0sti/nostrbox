//! Web UI extension: static file serving with SPA fallback.

use tower_http::services::{ServeDir, ServeFile};

/// Create an axum router that serves the web UI with SPA fallback.
pub fn webui_routes(dist_path: &str) -> axum::Router {
    let spa_fallback = ServeFile::new(format!("{dist_path}/index.html"));
    let serve_dir = ServeDir::new(dist_path).fallback(spa_fallback);
    axum::Router::new().fallback_service(serve_dir)
}
