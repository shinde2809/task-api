/// Integration tests for the Task Management API.
///
/// These tests use an in-memory SQLite DB so they are self-contained and
/// do not affect the development database.
use actix_web::{test, web, App};
use sqlx::sqlite::SqlitePoolOptions;

// Re-export app modules (needed because main.rs uses #[tokio::main])
use task_api_test_helpers::*;

// We expose a helper module so tests can build the App without duplicating setup.
// In the real binary this is done in main.rs.
mod task_api_test_helpers {
    use actix_web::{web, App};
    use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

    pub async fn build_test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory db");

        // Run DDL directly (same as migrations)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY, full_name TEXT NOT NULL, email TEXT NOT NULL UNIQUE,
                hashed_password TEXT NOT NULL, role TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            )",
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT, status TEXT NOT NULL,
                priority TEXT NOT NULL, created_by_id TEXT NOT NULL, assigned_to_id TEXT,
                created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            )",
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS login_challenges (
                id TEXT PRIMARY KEY, user_id TEXT NOT NULL, code_hash TEXT NOT NULL,
                used INTEGER NOT NULL DEFAULT 0, expires_at TEXT NOT NULL, created_at TEXT NOT NULL
            )",
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS email_logs (
                id TEXT PRIMARY KEY, to_email TEXT NOT NULL, subject TEXT NOT NULL,
                body TEXT NOT NULL, sent_at TEXT NOT NULL
            )",
        ).execute(&pool).await.unwrap();

        pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};

    /// Helper: parse JSON body from response.
    async fn parse_json(resp: actix_web::dev::ServiceResponse) -> serde_json::Value {
        let body = test::read_body(resp).await;
        serde_json::from_slice(&body).expect("response should be valid JSON")
    }

    #[tokio::test]
    async fn test_seed_creates_users() {
        dotenv::dotenv().ok();
        let pool = build_test_pool().await;
        use task_api::cache::TaskCache;
        use task_api::config::Config;
        use task_api::db::AppState;

        let state = web::Data::new(AppState {
            pool: pool.clone(),
            config: Config::from_env(),
            cache: TaskCache::new(),
        });

        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .route("/seed/users", web::post().to(task_api::handlers::seed::seed_users)),
        )
        .await;

        let req = test::TestRequest::post().uri("/seed/users").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "Seed should return 200");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 2, "Should have created exactly 2 users");
    }

    #[tokio::test]
    async fn test_login_returns_challenge_not_jwt() {
        dotenv::dotenv().ok();
        let pool = build_test_pool().await;
        use task_api::cache::TaskCache;
        use task_api::config::Config;
        use task_api::db::AppState;

        let state = web::Data::new(AppState {
            pool: pool.clone(),
            config: Config::from_env(),
            cache: TaskCache::new(),
        });

        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .route("/seed/users", web::post().to(task_api::handlers::seed::seed_users))
                .route("/auth/login", web::post().to(task_api::handlers::auth::login)),
        )
        .await;

        // Seed first
        let seed_req = test::TestRequest::post().uri("/seed/users").to_request();
        test::call_service(&app, seed_req).await;

        // Login
        let login_req = test::TestRequest::post()
            .uri("/auth/login")
            .set_json(serde_json::json!({
                "email": "admin@example.com",
                "password": "Admin1234!"
            }))
            .to_request();

        let resp = test::call_service(&app, login_req).await;
        assert_eq!(resp.status(), 200);

        let json = parse_json(resp).await;
        assert!(json["login_challenge_id"].is_string(), "Must return a challenge ID");
        assert!(json.get("access_token").is_none(), "Must NOT return a JWT at this stage");
    }

    #[tokio::test]
    async fn test_wrong_password_rejected() {
        dotenv::dotenv().ok();
        let pool = build_test_pool().await;
        use task_api::cache::TaskCache;
        use task_api::config::Config;
        use task_api::db::AppState;

        let state = web::Data::new(AppState {
            pool: pool.clone(),
            config: Config::from_env(),
            cache: TaskCache::new(),
        });

        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .route("/seed/users", web::post().to(task_api::handlers::seed::seed_users))
                .route("/auth/login", web::post().to(task_api::handlers::auth::login)),
        )
        .await;

        let seed_req = test::TestRequest::post().uri("/seed/users").to_request();
        test::call_service(&app, seed_req).await;

        let req = test::TestRequest::post()
            .uri("/auth/login")
            .set_json(serde_json::json!({
                "email": "admin@example.com",
                "password": "wrongpassword"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401, "Wrong password must be 401");
    }

    #[tokio::test]
    async fn test_staff_cannot_create_task() {
        // This test verifies the 403 path by calling create_task with a staff JWT.
        // We test the role enforcement logic directly.
        use task_api::middleware::auth::require_admin;
        use task_api::models::JwtClaims;

        let staff_claims = JwtClaims {
            sub: "some-id".to_string(),
            email: "jamesbond@example.com".to_string(),
            role: "staff".to_string(),
            exp: 9999999999,
        };

        let result = require_admin(&staff_claims);
        assert!(result.is_err(), "Staff must be forbidden from admin actions");

        let admin_claims = JwtClaims {
            sub: "admin-id".to_string(),
            email: "admin@example.com".to_string(),
            role: "admin".to_string(),
            exp: 9999999999,
        };

        let result = require_admin(&admin_claims);
        assert!(result.is_ok(), "Admin must pass the role check");
    }

    #[tokio::test]
    async fn test_2fa_code_hashing_and_verify() {
        use task_api::services::two_fa::{generate_code, hash_code, verify_code};

        let code = generate_code();
        assert_eq!(code.len(), 6, "OTP must be 6 digits");

        let hash = hash_code(&code);
        assert!(verify_code(&code, &hash), "Correct code must verify");
        assert!(!verify_code("000000", &hash), "Wrong code must not verify");
    }

    #[tokio::test]
    async fn test_cache_hit_miss_invalidate() {
        use task_api::cache::TaskCache;
        use task_api::models::{CachedTasks, TaskResponse};

        let cache = TaskCache::new();
        let user_id = "test-user-1";

        // Miss
        assert!(cache.get(user_id).is_none(), "Should be a miss initially");

        // Set
        cache.set(
            user_id,
            CachedTasks {
                tasks: vec![TaskResponse {
                    id: "t1".to_string(),
                    title: "Test task".to_string(),
                    status: "todo".to_string(),
                    priority: "high".to_string(),
                    assigned_to: Some("jamesbond@example.com".to_string()),
                }],
                user_email: "jamesbond@example.com".to_string(),
                user_role: "staff".to_string(),
            },
        );

        // Hit
        assert!(cache.get(user_id).is_some(), "Should be a hit after set");

        // Invalidate
        cache.invalidate(user_id);
        assert!(cache.get(user_id).is_none(), "Should be miss after invalidation");
    }
}
