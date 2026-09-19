use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ── User ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub full_name: String,
    pub email: String,
    pub hashed_password: String,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub fn new(full_name: &str, email: &str, hashed_password: &str, role: &str) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            full_name: full_name.to_string(),
            email: email.to_string(),
            hashed_password: hashed_password.to_string(),
            role: role.to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

// ── Task ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub created_by_id: String,
    pub assigned_to_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Task {
    pub fn new(
        title: &str,
        description: Option<&str>,
        priority: &str,
        created_by_id: &str,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            description: description.map(|s| s.to_string()),
            status: "todo".to_string(),
            priority: priority.to_string(),
            created_by_id: created_by_id.to_string(),
            assigned_to_id: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

// ── LoginChallenge ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LoginChallenge {
    pub id: String,
    pub user_id: String,
    pub code_hash: String,
    pub used: i64,
    pub expires_at: String,
    pub created_at: String,
}

impl LoginChallenge {
    pub fn new(user_id: &str, code_hash: &str, expiry_minutes: i64) -> Self {
        let now = Utc::now();
        let expires_at = (now + chrono::Duration::minutes(expiry_minutes)).to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            code_hash: code_hash.to_string(),
            used: 0,
            expires_at,
            created_at: now.to_rfc3339(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let expires = DateTime::parse_from_rfc3339(&self.expires_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Utc::now() > expires
    }
}

// ── EmailLog ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailLog {
    pub id: String,
    pub to_email: String,
    pub subject: String,
    pub body: String,
    pub sent_at: String,
}

impl EmailLog {
    pub fn new(to_email: &str, subject: &str, body: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            to_email: to_email.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            sent_at: Utc::now().to_rfc3339(),
        }
    }
}

// ── Request / Response DTOs ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginChallengeResponse {
    pub login_challenge_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct Verify2FARequest {
    pub login_challenge_id: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct AuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>, // "low" | "medium" | "high"
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct TaskResponse {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub assigned_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignTasksRequest {
    pub task_ids: Vec<String>,
    pub user_email: String,
}

#[derive(Debug, Serialize)]
pub struct MyTasksResponse {
    pub user: UserSummary,
    pub tasks: Vec<TaskResponse>,
    pub summary: TaskSummary,
    pub cache: CacheMeta,
}

#[derive(Debug, Serialize)]
pub struct UserSummary {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct TaskSummary {
    pub total_assigned_tasks: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheMeta {
    pub hit: bool,
}

#[derive(Debug, Serialize)]
pub struct SeedResponse {
    pub message: String,
    pub admin_email: String,
    pub staff_email: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

// ── JWT Claims ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,  // user_id
    pub email: String,
    pub role: String,
    pub exp: usize,
}

// ── Cached task list (stored in DashMap) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTasks {
    pub tasks: Vec<TaskResponse>,
    pub user_email: String,
    pub user_role: String,
}
