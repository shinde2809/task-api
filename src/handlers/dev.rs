use actix_web::{web, HttpResponse};

use crate::db::AppState;
use crate::errors::AppError;
use crate::models::EmailLog;

/// GET /dev/email-logs/latest
///
/// Development-only endpoint. Returns the most recently sent email log.
/// Use this to retrieve the 2FA code without a real inbox.
pub async fn latest_email_log(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let log = sqlx::query_as::<_, EmailLog>(
        "SELECT * FROM email_logs ORDER BY sent_at DESC LIMIT 1",
    )
    .fetch_optional(&state.pool)
    .await?;

    match log {
        Some(entry) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "id": entry.id,
            "to": entry.to_email,
            "subject": entry.subject,
            "body": entry.body,
            "sent_at": entry.sent_at,
            "note": "This endpoint is for local development only."
        }))),
        None => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "No emails have been sent yet."
        }))),
    }
}
