use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct RenderLimits {
    pub max_source_bytes: usize,
    pub max_asset_bytes: usize,
    pub max_html_bytes: usize,
    pub timeout: Duration,
}

impl Default for RenderLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 512 * 1024,
            max_asset_bytes: 5 * 1024 * 1024,
            max_html_bytes: 2 * 1024 * 1024,
            timeout: Duration::from_secs(10),
        }
    }
}
