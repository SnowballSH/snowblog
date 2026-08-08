mod auth;
mod config;
mod etag;
mod precondition;
mod problem;
mod routes;
mod state;

pub use config::Config;

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{any, get, post, put};
use axum::{Router, middleware};
use snowblog_core::render::Renderer;
use snowblog_core::service::BlogService;
use snowblog_core::store::Store;

use auth::AdminToken;
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

    let mut api = Router::new()
        .route("/health", get(routes::health::health))
        .route("/posts", get(routes::public::list_posts))
        .route("/posts/{slug}", get(routes::public::get_post))
        .route(
            "/posts/{slug}/assets/{*path}",
            get(routes::public::get_asset),
        );

    if let Some(token_file) = &config.admin_token_file {
        let token = AdminToken::load(token_file)?;
        let translation_body_limit = config.max_source_bytes * 2;
        api = api.nest(
            "/admin",
            admin_router(token, config.max_asset_bytes, translation_body_limit),
        );
    }

    let router = Router::new()
        .nest("/api/v1", api)
        .fallback(async || Problem::not_found())
        .layer(middleware::from_fn(problem::normalize_error_responses))
        .with_state(state);
    Ok(router)
}

fn admin_router(
    token: AdminToken,
    max_asset_bytes: usize,
    translation_body_limit: usize,
) -> Router<AppState> {
    use routes::admin;
    Router::new()
        .route("/posts", get(admin::list_posts).post(admin::create_post))
        .route(
            "/posts/{slug}",
            get(admin::get_post)
                .patch(admin::patch_post)
                .delete(admin::delete_post),
        )
        .route(
            "/posts/{slug}/translations/{language}",
            put(admin::put_translation)
                .delete(admin::delete_translation)
                .layer(DefaultBodyLimit::max(translation_body_limit)),
        )
        .route(
            "/posts/{slug}/assets/{*path}",
            put(admin::put_asset)
                .delete(admin::delete_asset)
                .layer(DefaultBodyLimit::max(max_asset_bytes)),
        )
        .route("/posts/{slug}/preview", post(admin::preview))
        .route("/posts/{slug}/publish", post(admin::publish))
        .route("/posts/{slug}/unpublish", post(admin::unpublish))
        .route("/posts/{slug}/archive", post(admin::archive))
        .route("/rerender", post(admin::rerender))
        .route("/", any(async || Problem::not_found()))
        .route("/{*unmatched}", any(async || Problem::not_found()))
        .layer(middleware::from_fn(move |request, next| {
            let token = token.clone();
            async move { auth::require_admin(token, request, next).await }
        }))
}
