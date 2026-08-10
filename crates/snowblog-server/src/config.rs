use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use clap::Args;
use snowblog_core::render::RenderLimits;

#[derive(Args, Clone, Debug)]
pub struct Config {
    #[arg(long, env = "SNOWBLOG_LISTEN", default_value = "127.0.0.1:8080")]
    pub listen: SocketAddr,
    #[arg(long, env = "SNOWBLOG_METRICS_LISTEN")]
    pub metrics_listen: Option<SocketAddr>,
    #[arg(long, env = "SNOWBLOG_DATABASE")]
    pub database: PathBuf,
    #[arg(long, env = "SNOWBLOG_ADMIN_TOKEN_FILE")]
    pub admin_token_file: Option<PathBuf>,
    #[arg(long, env = "SNOWBLOG_PACKAGE_ROOT", default_value = "vendor/packages")]
    pub package_root: PathBuf,
    #[arg(long = "font-dir", env = "SNOWBLOG_FONT_DIRS", value_delimiter = ':')]
    pub font_dirs: Vec<PathBuf>,
    #[arg(
        long,
        env = "SNOWBLOG_ASSET_URL_TEMPLATE",
        default_value = "/api/v1/posts/{slug}/assets/"
    )]
    pub asset_url_template: String,
    #[arg(long, env = "SNOWBLOG_MAX_SOURCE_BYTES", default_value_t = 512 * 1024)]
    pub max_source_bytes: usize,
    #[arg(long, env = "SNOWBLOG_MAX_ASSET_BYTES", default_value_t = 5 * 1024 * 1024)]
    pub max_asset_bytes: usize,
    #[arg(long, env = "SNOWBLOG_MAX_HTML_BYTES", default_value_t = 2 * 1024 * 1024)]
    pub max_html_bytes: usize,
    #[arg(long, env = "SNOWBLOG_RENDER_TIMEOUT_SECS", default_value_t = 10)]
    pub render_timeout_secs: u64,
}

impl Config {
    pub fn render_limits(&self) -> RenderLimits {
        RenderLimits {
            max_source_bytes: self.max_source_bytes,
            max_asset_bytes: self.max_asset_bytes,
            max_html_bytes: self.max_html_bytes,
            timeout: Duration::from_secs(self.render_timeout_secs),
        }
    }
}
