use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use clap::{Args, Subcommand};
use snowblog_core::import::{SourceAdaptation, import_dir};
use snowblog_core::render::{RenderLimits, Renderer};
use snowblog_core::service::{BlogService, RerenderScope};
use snowblog_core::store::Store;
use snowblog_server::Config;

#[derive(Args)]
pub struct ServeArgs {
    #[command(flatten)]
    config: Config,
}

pub async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    let listen = args.config.listen;
    let metrics = args
        .config
        .metrics_listen
        .map(|address| {
            snowblog_server::telemetry::install_prometheus_recorder()
                .map(|handle| (address, handle))
        })
        .transpose()?;
    let application = snowblog_server::build_application(args.config).await?;
    let api_listener = tokio::net::TcpListener::bind(listen).await?;

    let Some((metrics_listen, metrics_handle)) = metrics else {
        tracing::info!(%listen, "snowblog listening");
        axum::serve(api_listener, application.into_router())
            .await
            .context("API listener failed")?;
        anyhow::bail!("API listener terminated unexpectedly");
    };

    let metrics_listener = tokio::net::TcpListener::bind(metrics_listen).await?;
    snowblog_server::telemetry::initialize_build_info(application.service()).await?;
    snowblog_server::telemetry::refresh_content_metrics(application.service()).await?;

    let service = application.service().clone();
    let api = axum::serve(api_listener, application.into_router());
    let metrics = axum::serve(
        metrics_listener,
        snowblog_server::telemetry::metrics_router(metrics_handle),
    );
    tracing::info!(%listen, "snowblog listening");
    tracing::info!(%metrics_listen, "snowblog metrics listening");

    tokio::select! {
        result = api => {
            result.context("API listener failed")?;
            anyhow::bail!("API listener terminated unexpectedly");
        }
        result = metrics => {
            result.context("metrics listener failed")?;
            anyhow::bail!("metrics listener terminated unexpectedly");
        }
        () = reconcile_content_metrics(service) => {
            anyhow::bail!("metrics reconciliation terminated unexpectedly");
        }
    }
}

async fn reconcile_content_metrics(service: BlogService) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        if snowblog_server::telemetry::refresh_content_metrics(&service)
            .await
            .is_err()
        {
            tracing::warn!("content metrics refresh failed");
        }
    }
}

#[derive(Args)]
pub struct DbArgs {
    #[command(subcommand)]
    command: DbCommand,
}

#[derive(Subcommand)]
enum DbCommand {
    Migrate {
        #[arg(long, env = "SNOWBLOG_DATABASE")]
        database: PathBuf,
    },
}

pub async fn db(args: DbArgs) -> anyhow::Result<()> {
    match args.command {
        DbCommand::Migrate { database } => {
            Store::open(&database).await?;
            println!("migrations applied to {}", database.display());
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct SharedServiceArgs {
    #[arg(long, env = "SNOWBLOG_DATABASE")]
    database: PathBuf,
    #[arg(long, env = "SNOWBLOG_PACKAGE_ROOT", default_value = "vendor/packages")]
    package_root: PathBuf,
    #[arg(long = "font-dir", env = "SNOWBLOG_FONT_DIRS", value_delimiter = ':')]
    font_dirs: Vec<PathBuf>,
    #[arg(
        long,
        env = "SNOWBLOG_ASSET_URL_TEMPLATE",
        default_value = "/api/v1/posts/{slug}/assets/"
    )]
    asset_url_template: String,
    #[arg(long, env = "SNOWBLOG_MAX_SOURCE_BYTES", default_value_t = 512 * 1024)]
    max_source_bytes: usize,
    #[arg(long, env = "SNOWBLOG_MAX_ASSET_BYTES", default_value_t = 5 * 1024 * 1024)]
    max_asset_bytes: usize,
    #[arg(long, env = "SNOWBLOG_MAX_HTML_BYTES", default_value_t = 2 * 1024 * 1024)]
    max_html_bytes: usize,
    #[arg(long, env = "SNOWBLOG_RENDER_TIMEOUT_SECS", default_value_t = 10)]
    render_timeout_secs: u64,
}

impl SharedServiceArgs {
    async fn service(&self) -> anyhow::Result<BlogService> {
        let store = Store::open(&self.database).await?;
        let renderer = Arc::new(Renderer::new(
            self.package_root.clone(),
            self.font_dirs.clone(),
            RenderLimits {
                max_source_bytes: self.max_source_bytes,
                max_asset_bytes: self.max_asset_bytes,
                max_html_bytes: self.max_html_bytes,
                timeout: std::time::Duration::from_secs(self.render_timeout_secs),
            },
        ));
        Ok(BlogService::new(
            store,
            renderer,
            Some(self.asset_url_template.clone()),
        ))
    }
}

#[derive(Args)]
pub struct ImportArgs {
    #[command(flatten)]
    shared: SharedServiceArgs,
    #[arg(long)]
    dir: PathBuf,
    #[arg(long)]
    dry_run: bool,
}

pub async fn import(args: ImportArgs) -> anyhow::Result<()> {
    let service = args.shared.service().await?;
    let report = import_dir(
        &service,
        &args.dir,
        &SourceAdaptation::default(),
        args.dry_run,
    )
    .await?;
    for post in &report.imported {
        println!(
            "imported {} [{}] languages={} assets={}{}",
            post.slug,
            post.status,
            post.languages.join(","),
            post.assets,
            if post.render_failures.is_empty() {
                String::new()
            } else {
                format!(" render_failures={}", post.render_failures.join(","))
            }
        );
    }
    for slug in &report.skipped {
        println!("skipped {slug} (already present)");
    }
    for (name, error) in &report.failed {
        println!("failed {name}: {error}");
    }
    anyhow::ensure!(report.failed.is_empty(), "some posts failed to import");
    Ok(())
}

#[derive(Args)]
pub struct RerenderArgs {
    #[command(flatten)]
    shared: SharedServiceArgs,
    #[arg(long, value_enum, default_value = "stale")]
    scope: ScopeArg,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum ScopeArg {
    Stale,
    All,
}

pub async fn rerender(args: RerenderArgs) -> anyhow::Result<()> {
    let service = args.shared.service().await?;
    let scope = match args.scope {
        ScopeArg::Stale => RerenderScope::Stale,
        ScopeArg::All => RerenderScope::All,
    };
    let reports = service.rerender(scope).await?;
    for report in &reports {
        println!(
            "{} [{}]: {:?}",
            report.slug, report.language, report.outcome
        );
    }
    Ok(())
}
