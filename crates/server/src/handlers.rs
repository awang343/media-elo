use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use media_elo_core::{
    rating_to_elo, AddRequest, AddTypeRequest, EditRequest, RenameTypeRequest,
    ReorderTypesRequest, Row, UndoRequest, VoteRequest, VoteResponse, MAX_TYPE_LEN,
};
use serde_json::json;
use uuid::Uuid;

use crate::db;
use crate::AppState;

pub struct ApiError(StatusCode, String);

impl ApiError {
    fn bad(msg: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, msg.into())
    }
    fn not_found(msg: impl Into<String>) -> Self {
        Self(StatusCode::NOT_FOUND, msg.into())
    }
    fn conflict(msg: impl Into<String>) -> Self {
        Self(StatusCode::CONFLICT, msg.into())
    }
}

impl<E: std::fmt::Display> From<E> for ApiError {
    fn from(e: E) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

async fn run_blocking<F, T>(state: AppState, f: F) -> ApiResult<T>
where
    F: FnOnce(&mut rusqlite::Connection) -> anyhow::Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let mut conn = state.conn.lock().unwrap();
        f(&mut conn)
    })
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("join: {e}")))?
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn list_rows(State(state): State<AppState>) -> ApiResult<Json<Vec<Row>>> {
    let rows = run_blocking(state, |c| db::list_rows(c)).await?;
    Ok(Json(rows))
}

pub async fn add_row(
    State(state): State<AppState>,
    Json(req): Json<AddRequest>,
) -> ApiResult<Json<Row>> {
    let title = req.title.trim().to_string();
    if title.is_empty() {
        return Err(ApiError::bad("title is required"));
    }
    let elo = (rating_to_elo(req.rating) * 100.0).round() / 100.0;
    let type_ = req.type_;
    let status = req.status;
    let row = run_blocking(state, move |c| db::create_row(c, type_, title, elo, status)).await?;
    Ok(Json(row))
}

pub async fn edit_row(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<EditRequest>,
) -> ApiResult<Json<Row>> {
    let title = req.title.trim().to_string();
    if title.is_empty() {
        return Err(ApiError::bad("title is required"));
    }
    let row = run_blocking(state, move |c| db::update_row(c, id, &req.type_, &title, &req.status))
        .await?;
    row.map(Json).ok_or_else(|| ApiError::not_found("row"))
}

pub async fn delete_row(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let ok = run_blocking(state, move |c| db::delete_row(c, id)).await?;
    if ok {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("row"))
    }
}

#[derive(serde::Deserialize)]
pub struct SetStatusReq {
    pub status: String,
}

pub async fn set_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<SetStatusReq>,
) -> ApiResult<Json<Row>> {
    let row = run_blocking(state, move |c| db::set_status(c, id, &req.status)).await?;
    row.map(Json).ok_or_else(|| ApiError::not_found("row"))
}

pub async fn vote(
    State(state): State<AppState>,
    Json(req): Json<VoteRequest>,
) -> ApiResult<Json<VoteResponse>> {
    let outcome = run_blocking(state, move |c| db::apply_vote(c, req.winner_id, req.loser_id))
        .await?
        .ok_or_else(|| ApiError::not_found("row"))?;
    Ok(Json(VoteResponse {
        winner: outcome.winner,
        loser: outcome.loser,
        delta_winner: outcome.delta_winner,
        delta_loser: outcome.delta_loser,
    }))
}

pub async fn list_types(State(state): State<AppState>) -> ApiResult<Json<Vec<String>>> {
    let types = run_blocking(state, |c| db::list_types(c)).await?;
    Ok(Json(types))
}

fn validate_type_name(raw: &str) -> Result<String, ApiError> {
    let name = raw.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::bad("name is required"));
    }
    if name.chars().count() > MAX_TYPE_LEN {
        return Err(ApiError::bad(format!(
            "name must be at most {MAX_TYPE_LEN} characters"
        )));
    }
    Ok(name)
}

pub async fn add_type(
    State(state): State<AppState>,
    Json(req): Json<AddTypeRequest>,
) -> ApiResult<Json<Vec<String>>> {
    let name = validate_type_name(&req.name)?;
    let result = run_blocking(state, move |c| {
        let res = db::add_type(c, &name)?;
        let types = db::list_types(c)?;
        Ok((res, types))
    })
    .await?;
    match result {
        (db::AddTypeResult::Added, types) => Ok(Json(types)),
        (db::AddTypeResult::AlreadyExists, _) => Err(ApiError::conflict("type already exists")),
    }
}

pub async fn rename_type(
    State(state): State<AppState>,
    Path(old): Path<String>,
    Json(req): Json<RenameTypeRequest>,
) -> ApiResult<Json<Vec<String>>> {
    let new_name = validate_type_name(&req.new_name)?;
    let result = run_blocking(state, move |c| {
        let res = db::rename_type(c, &old, &new_name)?;
        let types = db::list_types(c)?;
        Ok((res, types))
    })
    .await?;
    match result {
        (db::RenameTypeResult::Renamed, types) => Ok(Json(types)),
        (db::RenameTypeResult::NotFound, _) => Err(ApiError::not_found("type")),
        (db::RenameTypeResult::Conflict, _) => Err(ApiError::conflict("target name already exists")),
    }
}

pub async fn reorder_types(
    State(state): State<AppState>,
    Json(req): Json<ReorderTypesRequest>,
) -> ApiResult<Json<Vec<String>>> {
    let names = req.names;
    let result = run_blocking(state, move |c| {
        let res = db::reorder_types(c, &names)?;
        let types = db::list_types(c)?;
        Ok((res, types))
    })
    .await?;
    match result {
        (db::ReorderTypesResult::Ok, types) => Ok(Json(types)),
        (db::ReorderTypesResult::Mismatch, _) => Err(ApiError::bad(
            "names must be a permutation of existing types",
        )),
    }
}

pub async fn delete_type(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<StatusCode> {
    let res = run_blocking(state, move |c| db::delete_type(c, &name)).await?;
    match res {
        db::DeleteTypeResult::Deleted => Ok(StatusCode::NO_CONTENT),
        db::DeleteTypeResult::NotFound => Err(ApiError::not_found("type")),
        db::DeleteTypeResult::InUse => Err(ApiError::conflict("type is in use")),
    }
}

pub async fn undo(
    State(state): State<AppState>,
    Json(req): Json<UndoRequest>,
) -> ApiResult<StatusCode> {
    let ok = run_blocking(state, move |c| {
        db::restore_vote(
            c,
            req.a_id,
            req.b_id,
            req.old_elo_a,
            req.old_elo_b,
            req.old_matches_a,
            req.old_matches_b,
        )
    })
    .await?;
    if ok {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("row"))
    }
}
