use anyhow::{Context, Result};
use media_elo_core::{today_str, Row, BASE_ELO, STATUS_BACKLOG, STATUS_DONE};
use rusqlite::Connection;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct RawRow {
    #[serde(rename = "type", default)]
    type_: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    elo: String,
    #[serde(default)]
    matches: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    date_added: String,
}

/// Returns number of rows imported. Renames the CSV to `<path>.bak` on success.
pub fn import_csv(conn: &mut Connection, csv_path: &Path) -> Result<usize> {
    let mut rdr = csv::Reader::from_path(csv_path)
        .with_context(|| format!("opening {}", csv_path.display()))?;
    let today = today_str();
    let mut imported = 0;
    let tx = conn.transaction()?;
    for raw in rdr.deserialize::<RawRow>() {
        let raw = raw?;
        let row = parse_raw(raw, &today);
        crate::db::insert_row(&tx, &row)?;
        imported += 1;
    }
    tx.commit()?;

    let bak = csv_path.with_extension("csv.bak");
    fs::rename(csv_path, &bak).with_context(|| {
        format!("renaming {} -> {}", csv_path.display(), bak.display())
    })?;
    Ok(imported)
}

fn parse_raw(r: RawRow, today: &str) -> Row {
    let matches = r.matches.parse().unwrap_or(0);
    let elo = if r.elo.is_empty() {
        BASE_ELO
    } else {
        r.elo.parse().unwrap_or(BASE_ELO)
    };
    let status = if r.status.is_empty() {
        STATUS_DONE.to_string()
    } else if r.status == "pending" {
        STATUS_BACKLOG.to_string()
    } else {
        r.status
    };
    let date_added = if r.date_added.is_empty() {
        today.to_string()
    } else {
        r.date_added
    };
    Row {
        id: Uuid::new_v4(),
        type_: r.type_,
        title: r.title,
        elo,
        matches,
        status,
        date_added,
    }
}
