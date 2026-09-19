use actix_web::{web, HttpResponse};
use chrono::Utc;

use crate::db::AppState;
use crate::errors::AppError;
use crate::middleware::auth::{require_admin, AuthUser};
use crate::models::{
    AssignTasksRequest, CachedTasks, CacheMeta, CreateTaskRequest, MyTasksResponse, Task,
    TaskResponse, TaskSummary, UserSummary,
};

/// POST /tasks
///
/// Admin only. Creates a single task.
pub async fn create_task(
    state: web::Data<AppState>,
    auth: AuthUser,
    body: web::Json<CreateTaskRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&auth.0)?;

    let priority = body
        .priority
        .as_deref()
        .unwrap_or("medium");

    // Validate priority
    if !["low", "medium", "high"].contains(&priority) {
        return Err(AppError::BadRequest(
            "priority must be 'low', 'medium', or 'high'".to_string(),
        ));
    }

    let task = Task::new(
        &body.title,
        body.description.as_deref(),
        priority,
        &auth.0.sub,
    );

    sqlx::query(
        "INSERT INTO tasks (id, title, description, status, priority, created_by_id, assigned_to_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&task.id)
    .bind(&task.title)
    .bind(&task.description)
    .bind(&task.status)
    .bind(&task.priority)
    .bind(&task.created_by_id)
    .bind(&task.assigned_to_id)
    .bind(&task.created_at)
    .bind(&task.updated_at)
    .execute(&state.pool)
    .await?;

    println!("✅ Task created: '{}' (id: {})", task.title, task.id);

    Ok(HttpResponse::Created().json(serde_json::json!({
        "id": task.id,
        "title": task.title,
        "status": task.status,
        "priority": task.priority,
        "message": "Task created successfully"
    })))
}

/// POST /tasks/assign
///
/// Admin only. Assigns a list of task IDs to a user by email.
/// Invalidates that user's task cache after assignment.
pub async fn assign_tasks(
    state: web::Data<AppState>,
    auth: AuthUser,
    body: web::Json<AssignTasksRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&auth.0)?;

    let pool = &state.pool;

    // Resolve target user
    let target_user = sqlx::query_as::<_, crate::models::User>(
        "SELECT * FROM users WHERE email = ?",
    )
    .bind(&body.user_email)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("User '{}' not found", body.user_email)))?;

    let now = Utc::now().to_rfc3339();
    let mut assigned_count = 0usize;

    for task_id in &body.task_ids {
        let rows = sqlx::query(
            "UPDATE tasks SET assigned_to_id = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&target_user.id)
        .bind(&now)
        .bind(task_id)
        .execute(pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(AppError::NotFound(format!("Task '{task_id}' not found")));
        }
        assigned_count += 1;
    }

    // Invalidate cache for the target user
    state.cache.invalidate(&target_user.id);
    println!(
        "🗑️  Cache invalidated for user: {} (tasks reassigned)",
        target_user.email
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": format!("{assigned_count} task(s) assigned to {}", body.user_email),
        "assigned_to": body.user_email,
        "task_ids": body.task_ids
    })))
}

/// GET /tasks/view-my-tasks
///
/// Returns tasks assigned to the authenticated user.
/// First call: loads from DB, caches result, returns cache.hit = false.
/// Second call: returns from cache, cache.hit = true.
pub async fn view_my_tasks(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, AppError> {
    let user_id = &auth.0.sub;
    let pool = &state.pool;

    // ── Cache check ──────────────────────────────────────────────────────────
    if let Some(cached) = state.cache.get(user_id) {
        println!("⚡ Cache HIT for user: {}", auth.0.email);
        let tasks: Vec<TaskResponse> = cached
            .tasks
            .into_iter()
            .map(|t| TaskResponse {
                id: t.id,
                title: t.title,
                status: t.status,
                priority: t.priority,
                assigned_to: t.assigned_to,
            })
            .collect();

        let total = tasks.len();
        return Ok(HttpResponse::Ok().json(MyTasksResponse {
            user: UserSummary {
                email: cached.user_email,
                role: cached.user_role,
            },
            tasks,
            summary: TaskSummary {
                total_assigned_tasks: total,
            },
            cache: CacheMeta { hit: true },
        }));
    }

    // ── DB load ──────────────────────────────────────────────────────────────
    println!("💾 Cache MISS for user: {} — loading from DB", auth.0.email);

    // Get user email for assigned_to field
    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    let tasks_raw = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE assigned_to_id = ? ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let tasks: Vec<TaskResponse> = tasks_raw
        .iter()
        .map(|t| TaskResponse {
            id: t.id.clone(),
            title: t.title.clone(),
            status: t.status.clone(),
            priority: t.priority.clone(),
            assigned_to: Some(user.email.clone()),
        })
        .collect();

    let total = tasks.len();

    // ── Populate cache ───────────────────────────────────────────────────────
    state.cache.set(
        user_id,
        CachedTasks {
            tasks: tasks
                .iter()
                .map(|t| crate::models::TaskResponse {
                    id: t.id.clone(),
                    title: t.title.clone(),
                    status: t.status.clone(),
                    priority: t.priority.clone(),
                    assigned_to: t.assigned_to.clone(),
                })
                .collect(),
            user_email: user.email.clone(),
            user_role: user.role.clone(),
        },
    );

    Ok(HttpResponse::Ok().json(MyTasksResponse {
        user: UserSummary {
            email: user.email,
            role: user.role,
        },
        tasks,
        summary: TaskSummary {
            total_assigned_tasks: total,
        },
        cache: CacheMeta { hit: false },
    }))
}
