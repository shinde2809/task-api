use actix_web::{middleware::Logger, web, App, HttpServer};
use sqlx::sqlite::SqlitePoolOptions;

mod cache;
mod config;
mod db;
mod errors;
mod handlers;
mod middleware;
mod models;
mod services;

use cache::TaskCache;
use config::Config;
use db::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let config = Config::from_env();

    // ── Database ─────────────────────────────────────────────────────────────
       // Auto-create the SQLite file if it does not exist
    let db_file = config.database_url
        .trim_start_matches("sqlite://")
        .split('?')
        .next()
        .unwrap_or("task_api.db");
    if !std::path::Path::new(db_file).exists() {
        std::fs::File::create(db_file).expect("Could not create SQLite file");
        println!("📁 Created database file: {db_file}");
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to SQLite");

    // Run embedded migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    println!("✅ Database connected and migrations applied.");

    // ── Shared state ──────────────────────────────────────────────────────────
    let state = web::Data::new(AppState {
        pool,
        config: config.clone(),
        cache: TaskCache::new(),
    });

    let bind_addr = format!("{}:{}", config.server_host, config.server_port);
    println!("🚀 Task API running at http://{bind_addr}");
    println!("📖 See README.md for the full validation workflow.");

    // ── Server ────────────────────────────────────────────────────────────────
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .app_data(
                web::JsonConfig::default()
                    .error_handler(|err, _req| {
                        let response = actix_web::HttpResponse::BadRequest().json(
                            models::ApiError {
                                error: "bad_request".to_string(),
                                message: format!("JSON error: {err}"),
                            },
                        );
                        actix_web::error::InternalError::from_response(err, response).into()
                    }),
            )
            // Seed
            .route("/seed/users", web::post().to(handlers::seed::seed_users))
            // Auth
            .route("/auth/login", web::post().to(handlers::auth::login))
            .route("/auth/verify-2fa", web::post().to(handlers::auth::verify_2fa))
            // Tasks
            .route("/tasks", web::post().to(handlers::tasks::create_task))
            .route("/tasks/assign", web::post().to(handlers::tasks::assign_tasks))
            .route("/tasks/view-my-tasks", web::get().to(handlers::tasks::view_my_tasks))
            // Dev tools
            .route("/dev/email-logs/latest", web::get().to(handlers::dev::latest_email_log))
    })
    .bind(&bind_addr)?
    .run()
    .await?;

    Ok(())
}
