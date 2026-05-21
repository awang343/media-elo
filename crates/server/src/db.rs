use anyhow::{Context, Result};
use media_elo_core::{apply_match, today_str, Row};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS rows (
    id          TEXT PRIMARY KEY,
    type        TEXT NOT NULL,
    title       TEXT NOT NULL,
    elo         REAL NOT NULL,
    matches     INTEGER NOT NULL,
    status      TEXT NOT NULL,
    date_added  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS rows_status ON rows(status);
CREATE INDEX IF NOT EXISTS rows_type   ON rows(type);

CREATE TABLE IF NOT EXISTS types (
    name           TEXT NOT NULL PRIMARY KEY COLLATE NOCASE,
    display_order  INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS types_name_nocase ON types(name COLLATE NOCASE);
"#;

pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(SCHEMA).context("creating schema")?;
    Ok(())
}

pub fn count(conn: &Connection) -> Result<i64> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM rows", [], |r| r.get(0))?;
    Ok(n)
}

pub fn seed_types_if_empty(conn: &Connection, defaults: &[&str]) -> Result<()> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM types", [], |r| r.get(0))?;
    if n > 0 {
        return Ok(());
    }
    for (i, name) in defaults.iter().enumerate() {
        conn.execute(
            "INSERT INTO types (name, display_order) VALUES (?1, ?2)",
            params![name, i as i64],
        )?;
    }
    Ok(())
}

pub fn list_types(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT name FROM types ORDER BY display_order, name")?;
    let names = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(names)
}

pub enum AddTypeResult {
    Added,
    AlreadyExists,
}

/// Case-insensitive uniqueness check + insert. Caller is expected to have
/// trimmed `name` and bounded its length.
pub fn add_type(conn: &Connection, name: &str) -> Result<AddTypeResult> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM types WHERE name = ?1 COLLATE NOCASE",
        params![name],
        |r| r.get(0),
    )?;
    if exists > 0 {
        return Ok(AddTypeResult::AlreadyExists);
    }
    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(display_order), -1) + 1 FROM types",
        [],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO types (name, display_order) VALUES (?1, ?2)",
        params![name, next],
    )?;
    Ok(AddTypeResult::Added)
}

pub enum RenameTypeResult {
    Renamed,
    NotFound,
    Conflict,
}

/// Renames a type and cascades the change to `rows.type`. Case-only changes
/// (e.g. `Manga` → `manga`) are allowed; collisions with a different existing
/// type are reported as `Conflict`.
pub fn rename_type(conn: &mut Connection, old: &str, new: &str) -> Result<RenameTypeResult> {
    let tx = conn.transaction()?;
    let conflict: i64 = tx.query_row(
        "SELECT COUNT(*) FROM types WHERE name = ?1 COLLATE NOCASE AND name <> ?2 COLLATE NOCASE",
        params![new, old],
        |r| r.get(0),
    )?;
    if conflict > 0 {
        return Ok(RenameTypeResult::Conflict);
    }
    let n = tx.execute(
        "UPDATE types SET name = ?1 WHERE name = ?2 COLLATE NOCASE",
        params![new, old],
    )?;
    if n == 0 {
        return Ok(RenameTypeResult::NotFound);
    }
    tx.execute(
        "UPDATE rows SET type = ?1 WHERE type = ?2 COLLATE NOCASE",
        params![new, old],
    )?;
    tx.commit()?;
    Ok(RenameTypeResult::Renamed)
}

pub enum ReorderTypesResult {
    Ok,
    Mismatch,
}

/// Sets `display_order` to match the position of each name in `names`. The
/// provided list must be a permutation of the current types (case-insensitive).
pub fn reorder_types(conn: &mut Connection, names: &[String]) -> Result<ReorderTypesResult> {
    let current = list_types(conn)?;
    if names.len() != current.len() {
        return Ok(ReorderTypesResult::Mismatch);
    }
    let mut remaining: std::collections::HashSet<String> =
        current.iter().map(|s| s.to_lowercase()).collect();
    for n in names {
        if !remaining.remove(&n.to_lowercase()) {
            return Ok(ReorderTypesResult::Mismatch);
        }
    }
    let tx = conn.transaction()?;
    for (i, name) in names.iter().enumerate() {
        tx.execute(
            "UPDATE types SET display_order = ?1 WHERE name = ?2 COLLATE NOCASE",
            params![i as i64, name],
        )?;
    }
    tx.commit()?;
    Ok(ReorderTypesResult::Ok)
}

pub enum DeleteTypeResult {
    Deleted,
    NotFound,
    InUse,
}

pub fn delete_type(conn: &Connection, name: &str) -> Result<DeleteTypeResult> {
    let in_use: i64 = conn.query_row(
        "SELECT COUNT(*) FROM rows WHERE type = ?1 COLLATE NOCASE",
        params![name],
        |r| r.get(0),
    )?;
    if in_use > 0 {
        return Ok(DeleteTypeResult::InUse);
    }
    let n = conn.execute(
        "DELETE FROM types WHERE name = ?1 COLLATE NOCASE",
        params![name],
    )?;
    Ok(if n == 0 {
        DeleteTypeResult::NotFound
    } else {
        DeleteTypeResult::Deleted
    })
}

pub fn insert_row(conn: &Connection, r: &Row) -> Result<()> {
    conn.execute(
        "INSERT INTO rows (id, type, title, elo, matches, status, date_added)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            r.id.to_string(),
            r.type_,
            r.title,
            r.elo,
            r.matches,
            r.status,
            r.date_added,
        ],
    )?;
    Ok(())
}

pub fn list_rows(conn: &Connection) -> Result<Vec<Row>> {
    let mut stmt = conn.prepare(
        "SELECT id, type, title, elo, matches, status, date_added FROM rows",
    )?;
    let rows = stmt
        .query_map([], row_from_sql)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get_row(conn: &Connection, id: Uuid) -> Result<Option<Row>> {
    let r = conn
        .query_row(
            "SELECT id, type, title, elo, matches, status, date_added FROM rows WHERE id = ?1",
            params![id.to_string()],
            row_from_sql,
        )
        .optional()?;
    Ok(r)
}

pub fn delete_row(conn: &Connection, id: Uuid) -> Result<bool> {
    let n = conn.execute("DELETE FROM rows WHERE id = ?1", params![id.to_string()])?;
    Ok(n > 0)
}

pub fn create_row(
    conn: &Connection,
    type_: String,
    title: String,
    elo: f64,
    status: String,
) -> Result<Row> {
    let r = Row {
        id: Uuid::new_v4(),
        type_,
        title,
        elo,
        matches: 0,
        status,
        date_added: today_str(),
    };
    insert_row(conn, &r)?;
    Ok(r)
}

pub fn update_row(
    conn: &Connection,
    id: Uuid,
    type_: &str,
    title: &str,
    status: &str,
) -> Result<Option<Row>> {
    let n = conn.execute(
        "UPDATE rows SET type = ?1, title = ?2, status = ?3 WHERE id = ?4",
        params![type_, title, status, id.to_string()],
    )?;
    if n == 0 {
        return Ok(None);
    }
    get_row(conn, id)
}

pub fn set_status(conn: &Connection, id: Uuid, status: &str) -> Result<Option<Row>> {
    let n = conn.execute(
        "UPDATE rows SET status = ?1 WHERE id = ?2",
        params![status, id.to_string()],
    )?;
    if n == 0 {
        return Ok(None);
    }
    get_row(conn, id)
}

pub struct VoteOutcome {
    pub winner: Row,
    pub loser: Row,
    pub delta_winner: f64,
    pub delta_loser: f64,
}

pub fn apply_vote(conn: &mut Connection, winner_id: Uuid, loser_id: Uuid) -> Result<Option<VoteOutcome>> {
    if winner_id == loser_id {
        anyhow::bail!("winner and loser must differ");
    }
    let tx = conn.transaction()?;
    let Some(w_before) = get_row(&tx, winner_id)? else {
        return Ok(None);
    };
    let Some(l_before) = get_row(&tx, loser_id)? else {
        return Ok(None);
    };
    let (new_w_elo, new_l_elo) =
        apply_match(w_before.elo, l_before.elo, w_before.matches, l_before.matches, true);
    tx.execute(
        "UPDATE rows SET elo = ?1, matches = matches + 1 WHERE id = ?2",
        params![new_w_elo, winner_id.to_string()],
    )?;
    tx.execute(
        "UPDATE rows SET elo = ?1, matches = matches + 1 WHERE id = ?2",
        params![new_l_elo, loser_id.to_string()],
    )?;
    let winner = get_row(&tx, winner_id)?.expect("winner row vanished mid-tx");
    let loser = get_row(&tx, loser_id)?.expect("loser row vanished mid-tx");
    tx.commit()?;
    Ok(Some(VoteOutcome {
        delta_winner: winner.elo - w_before.elo,
        delta_loser: loser.elo - l_before.elo,
        winner,
        loser,
    }))
}

pub fn restore_vote(
    conn: &mut Connection,
    a_id: Uuid,
    b_id: Uuid,
    elo_a: f64,
    elo_b: f64,
    matches_a: u32,
    matches_b: u32,
) -> Result<bool> {
    let tx = conn.transaction()?;
    let na = tx.execute(
        "UPDATE rows SET elo = ?1, matches = ?2 WHERE id = ?3",
        params![elo_a, matches_a, a_id.to_string()],
    )?;
    let nb = tx.execute(
        "UPDATE rows SET elo = ?1, matches = ?2 WHERE id = ?3",
        params![elo_b, matches_b, b_id.to_string()],
    )?;
    tx.commit()?;
    Ok(na > 0 && nb > 0)
}

fn row_from_sql(r: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    let id_str: String = r.get(0)?;
    let id = Uuid::parse_str(&id_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(Row {
        id,
        type_: r.get(1)?,
        title: r.get(2)?,
        elo: r.get(3)?,
        matches: r.get::<_, i64>(4)? as u32,
        status: r.get(5)?,
        date_added: r.get(6)?,
    })
}
