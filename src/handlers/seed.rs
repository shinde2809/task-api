use actix_web::{web, HttpResponse};

use crate::db::AppState;
use crate::errors::AppError;
use crate::models::{SeedResponse, User};
use crate::services::password::hash_password;

/// POST /seed/users
///
/// Idempotent: skips creation if the email already exists.
/// Creates:
///   - admin@example.com  / password: Admin1234!  / role: admin
///   - jamesbond@example.com / password: Bond007!  / role: staff
pub async fn seed_users(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let pool = &state.pool;

    upsert_user(
        pool,
        "Admin User",
        "admin@example.com",
        "Admin1234!",
        "admin",
    )
    .await?;

    upsert_user(
        pool,
        "James Bond",
        "jamesbond@example.com",
        "Bond007!",
        "staff",
    )
    .await?;

    Ok(HttpResponse::Ok().json(SeedResponse {
        message: "Seed complete. Users created (or already existed).".to_string(),
        admin_email: "admin@example.com".to_string(),
        staff_email: "jamesbond@example.com".to_string(),
    }))
}

async fn upsert_user(
    pool: &sqlx::SqlitePool,
    full_name: &str,
    email: &str,
    password: &str,
    role: &str,
) -> Result<(), AppError> {
    // Skip if already exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = ?)")
        .bind(email)
        .fetch_one(pool)
        .await?;

    if exists {
        println!("ℹ️  User {email} already exists, skipping.");
        return Ok(());
    }

    let hashed = hash_password(password)?;
    let user = User::new(full_name, email, &hashed, role);

    sqlx::query(
        "INSERT INTO users (id, full_name, email, hashed_password, role, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user.id)
    .bind(&user.full_name)
    .bind(&user.email)
    .bind(&user.hashed_password)
    .bind(&user.role)
    .bind(&user.created_at)
    .bind(&user.updated_at)
    .execute(pool)
    .await?;

    println!("✅ Created user: {email} (role: {role})");
    Ok(())
}
