use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const STATUS_DONE: &str = "done";
pub const STATUS_IN_PROGRESS: &str = "in progress";
pub const STATUS_ON_HOLD: &str = "on hold";
pub const STATUS_DROPPED: &str = "dropped";
pub const STATUS_BACKLOG: &str = "backlog";

pub const STATUSES: &[&str] = &[
    STATUS_BACKLOG,
    STATUS_IN_PROGRESS,
    STATUS_ON_HOLD,
    STATUS_DONE,
    STATUS_DROPPED,
];

/// Seeded into the server's `types` table on first run. After that, the
/// authoritative list lives in the DB and is fetched by clients at startup.
pub const DEFAULT_TYPES: &[&str] = &[
    "Web Novel",
    "Fanfic",
    "Manga",
    "Book",
    "Show",
    "Movie",
    "Visual Novel",
    "Game",
];

pub const BASE_ELO: f64 = 1500.0;

/// Upper bound on user-supplied type names (after trimming).
pub const MAX_TYPE_LEN: usize = 64;

pub fn is_rankable(status: &str) -> bool {
    status == STATUS_DONE || status == STATUS_DROPPED
}

pub fn today_str() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub type_: String,
    pub title: String,
    pub elo: f64,
    pub matches: u32,
    pub status: String,
    #[serde(default)]
    pub date_added: String,
}

// ----------------------------
// Elo math
// ----------------------------
pub fn expected(a: f64, b: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf((b - a) / 400.0))
}

pub fn get_k(matches: u32) -> f64 {
    if matches < 10 {
        32.0
    } else if matches < 30 {
        24.0
    } else {
        16.0
    }
}

pub fn update_elo(a: f64, b: f64, score_a: f64, k: f64) -> (f64, f64) {
    let ea = expected(a, b);
    let eb = expected(b, a);
    (a + k * (score_a - ea), b + k * ((1.0 - score_a) - eb))
}

pub fn rating_to_elo(rating: Option<f64>) -> f64 {
    match rating {
        None => BASE_ELO,
        Some(r) => 1200.0 + (r - 1.0) * (600.0 / 9.0),
    }
}

/// Returns new (elo_a, elo_b) after a single match, rounded to 2 decimals.
pub fn apply_match(elo_a: f64, elo_b: f64, matches_a: u32, matches_b: u32, a_wins: bool) -> (f64, f64) {
    let k = (get_k(matches_a) + get_k(matches_b)) / 2.0;
    let score_a = if a_wins { 1.0 } else { 0.0 };
    let (new_a, new_b) = update_elo(elo_a, elo_b, score_a, k);
    (
        (new_a * 100.0).round() / 100.0,
        (new_b * 100.0).round() / 100.0,
    )
}

// ----------------------------
// Wire DTOs shared by client and server
// ----------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddRequest {
    #[serde(rename = "type")]
    pub type_: String,
    pub title: String,
    pub rating: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditRequest {
    #[serde(rename = "type")]
    pub type_: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    pub winner_id: Uuid,
    pub loser_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponse {
    pub winner: Row,
    pub loser: Row,
    pub delta_winner: f64,
    pub delta_loser: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTypeRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameTypeRequest {
    pub new_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderTypesRequest {
    pub names: Vec<String>,
}

/// Client owns the undo stack and sends back the pre-vote snapshot to restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoRequest {
    pub a_id: Uuid,
    pub b_id: Uuid,
    pub old_elo_a: f64,
    pub old_elo_b: f64,
    pub old_matches_a: u32,
    pub old_matches_b: u32,
}
