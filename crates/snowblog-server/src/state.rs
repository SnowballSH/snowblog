use snowblog_core::service::BlogService;

#[derive(Clone)]
pub struct AppState {
    pub service: BlogService,
}
