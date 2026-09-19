use actix_web::{web, HttpResponse};
use chrono::Utc;

use crate::db::AppState;
use crate::errors::AppError;
use crate::models::{
    AuthTokenResponse, LoginChallengeResponse, LoginChallenge, LoginRequest, Verify2FARequest,
};
use crate::services::{
    email::send_verification_email,
    jwt::create_jwt,
    password::verify_password,
    two_fa::{generate_code, hash_code, verify_code},
};

/// POST /auth/login
///
/// Step 1 of login:
///   - Validates email + password
///   - Generates a 6-digit OTP, hashes it, stores in login_challenges
///   - Sends the OTP via email (console + DB log in dev)
///   - Returns login_challenge_id (NOT a JWT)
pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    let pool = &state.pool;

    // 1. Look up user
    let user = sqlx::query_as::<_, crate::models::User>(
        "SELECT * FROM users WHERE email = ?",
    )
    .bind(&body.email)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

    // 2. Verify password
    if !verify_password(&body.password, &user.hashed_password)? {
        return Err(AppError::Unauthorized("Invalid email or password".to_string()));
    }

    // 3. Generate OTP
    let code = generate_code();
    let code_hash = hash_code(&code);

    // 4. Persist challenge
    let challenge = LoginChallenge::new(
        &user.id,
        &code_hash,
        state.config.two_fa_expiry_minutes,
    );

    sqlx::query(
        "INSERT INTO login_challenges (id, user_id, code_hash, used, expires_at, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&challenge.id)
    .bind(&challenge.user_id)
    .bind(&challenge.code_hash)
    .bind(challenge.used)
    .bind(&challenge.expires_at)
    .bind(&challenge.created_at)
    .execute(pool)
    .await?;

    // 5. Send email (console + DB in dev)
    send_verification_email(pool, &user.email, &code).await?;

    Ok(HttpResponse::Ok().json(LoginChallengeResponse {
        login_challenge_id: challenge.id,
        message: format!(
            "Verification code sent to {}. Check console or GET /dev/email-logs/latest",
            user.email
        ),
    }))
}

/// POST /auth/verify-2fa
///
/// Step 2 of login:
///   - Validates the challenge ID exists and is not used/expired
///   - Verifies the submitted OTP against the stored hash
///   - Marks the challenge as used (one-time)
///   - Issues a JWT
pub async fn verify_2fa(
    state: web::Data<AppState>,
    body: web::Json<Verify2FARequest>,
) -> Result<HttpResponse, AppError> {
    let pool = &state.pool;

    // 1. Load challenge
    let challenge = sqlx::query_as::<_, LoginChallenge>(
        "SELECT * FROM login_challenges WHERE id = ?",
    )
    .bind(&body.login_challenge_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("Challenge not found".to_string()))?;

    // 2. Already used?
    if challenge.used != 0 {
        return Err(AppError::BadRequest("Code has already been used".to_string()));
    }

    // 3. Expired?
    if challenge.is_expired() {
        return Err(AppError::BadRequest("Code has expired".to_string()));
    }

    // 4. Verify OTP
    if !verify_code(&body.code, &challenge.code_hash) {
        return Err(AppError::Unauthorized("Invalid verification code".to_string()));
    }

    // 5. Mark as used (prevents replay)
    sqlx::query("UPDATE login_challenges SET used = 1 WHERE id = ?")
        .bind(&challenge.id)
        .execute(pool)
        .await?;

    // 6. Load user to build JWT claims
    let user = sqlx::query_as::<_, crate::models::User>(
        "SELECT * FROM users WHERE id = ?",
    )
    .bind(&challenge.user_id)
    .fetch_one(pool)
    .await?;

    // 7. Issue JWT
    let token = create_jwt(
        &user.id,
        &user.email,
        &user.role,
        &state.config.jwt_secret,
        state.config.jwt_expiry_hours,
    )?;

    Ok(HttpResponse::Ok().json(AuthTokenResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
    }))
}
