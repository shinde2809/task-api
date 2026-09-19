use sqlx::SqlitePool;

use crate::cache::TaskCache;
use crate::config::Config;

/// Shared application state injected into every handler via `web::Data<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Config,
    pub cache: TaskCache,
}
