use std::path::PathBuf;
use std::sync::Arc;

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
    let app = snowblog_server::build_app(args.config).await?;
    let listener = tokio::net::TcpListener::bind(listen).await?;
    tracing::info!(%listen, "snowblog listening");
    axum::serve(listener, app).await?;
    Ok(())
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
    #[arg(
        long,
        env = "SNOWBLOG_ASSET_URL_TEMPLATE",
        default_value = "/api/v1/posts/{slug}/assets/"
    )]
    asset_url_template: String,
}

impl SharedServiceArgs {
    async fn service(&self) -> anyhow::Result<BlogService> {
        let store = Store::open(&self.database).await?;
        let renderer = Arc::new(Renderer::new(
            self.package_root.clone(),
            Vec::new(),
            RenderLimits::default(),
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
