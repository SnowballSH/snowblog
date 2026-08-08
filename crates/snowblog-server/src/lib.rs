mod config;
mod etag;
mod problem;
mod routes;
mod state;

pub use config::Config;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use snowblog_core::render::Renderer;
use snowblog_core::service::BlogService;
use snowblog_core::store::Store;

use problem::Problem;
use state::AppState;

pub async fn build_app(config: Config) -> anyhow::Result<Router> {
    let store = Store::open(&config.database).await?;
    let renderer = Arc::new(Renderer::new(
        config.package_root.clone(),
        config.font_dirs.clone(),
        config.render_limits(),
    ));
    let service = BlogService::new(store, renderer, Some(config.asset_url_template.clone()));
    let state = AppState { service };

    let api = Router::new()
        .route("/health", get(routes::health::health))
        .route("/posts", get(routes::public::list_posts))
        .route("/posts/{slug}", get(routes::public::get_post))
        .route(
            "/posts/{slug}/assets/{*path}",
            get(routes::public::get_asset),
        );

    let router = Router::new()
        .nest("/api/v1", api)
        .fallback(async || Problem::not_found())
        .with_state(state);
    Ok(router)
}
