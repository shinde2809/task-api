use sqlx::SqlitePool;

use crate::errors::AppError;
use crate::models::EmailLog;

/// Simulates email delivery by:
///   1. Printing clearly to stdout (visible in `cargo run` terminal)
///   2. Persisting to `email_logs` table (readable via GET /dev/email-logs/latest)
///
/// Replace this function body with real SMTP (lettre crate) or Mailtrap for production.
pub async fn send_verification_email(
    pool: &SqlitePool,
    to_email: &str,
    code: &str,
) -> Result<(), AppError> {
    let subject = "Your verification code";
    let body = format!(
        "Hello,\n\nYour two-factor authentication code is: {code}\n\nIt expires in 5 minutes.\n\nDo not share this code."
    );

    // Console log — always visible during local dev
    println!("\n┌─────────────────────────────────────────┐");
    println!("│         📧  VERIFICATION EMAIL           │");
    println!("├─────────────────────────────────────────┤");
    println!("│  To:      {to_email:<31}│");
    println!("│  Subject: {subject:<31}│");
    println!("│  Code:    {code:<31}│");
    println!("└─────────────────────────────────────────┘\n");

    // Persist to DB for /dev/email-logs/latest
    let log = EmailLog::new(to_email, subject, &body);
    sqlx::query(
        "INSERT INTO email_logs (id, to_email, subject, body, sent_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&log.id)
    .bind(&log.to_email)
    .bind(&log.subject)
    .bind(&log.body)
    .bind(&log.sent_at)
    .execute(pool)
    .await?;

    Ok(())
}
