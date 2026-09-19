use actix_web::{dev::Payload, web, FromRequest, HttpRequest};
use futures::future::{ready, Ready};
use std::env;

use crate::errors::AppError;
use crate::models::JwtClaims;
use crate::services::jwt::verify_jwt;

/// Extractor: pulls and validates the Bearer JWT from the Authorization header.
/// Use as a handler parameter: `claims: AuthUser`
pub struct AuthUser(pub JwtClaims);

impl FromRequest for AuthUser {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let result = extract_claims(req);
        ready(result)
    }
}

fn extract_claims(req: &HttpRequest) -> Result<AuthUser, AppError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Authorization header must start with 'Bearer '".to_string()))?;

    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-me".to_string());
    let claims = verify_jwt(token, &secret)?;
    Ok(AuthUser(claims))
}

/// Guard helper: returns Forbidden if the user role is not admin.
pub fn require_admin(claims: &JwtClaims) -> Result<(), AppError> {
    if claims.role != "admin" {
        return Err(AppError::Forbidden(
            "Only admins can perform this action".to_string(),
        ));
    }
    Ok(())
}
