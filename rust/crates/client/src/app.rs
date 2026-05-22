use crate::api::Api;
use crate::pairer::Pairer;
use anyhow::Result;
use media_elo_core::{
    is_rankable, AddRequest, EditRequest, Row, UndoRequest, MAX_TYPE_LEN, STATUSES,
    STATUS_BACKLOG, STATUS_DONE,
};
use std::process::{Command, Stdio};
use uuid::Uuid;

/// Mirrors server-side validation so the modal can show errors without
/// round-tripping; the server is still authoritative.
fn validate_type_name(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some("name is required".to_string());
    }
    if name.chars().count() > MAX_TYPE_LEN {
        return Some(format!("max {MAX_TYPE_LEN} chars"));
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    List,
    Compare,
    Stats,
}

pub enum Modal {
    None,
    Add(AddModal),
    Edit(EditModal),
    Confirm(ConfirmModal),
    TypeManager(TypeManagerModal),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TypeManagerMode {
    Browse,
    AddInput,
    RenameInput,
}

pub struct TypeManagerModal {
    pub cursor: usize,
    pub mode: TypeManagerMode,
    pub buffer: String,
    pub error: Option<String>,
}

impl Default for TypeManagerModal {
    fn default() -> Self {
        Self {
            cursor: 0,
            mode: TypeManagerMode::Browse,
            buffer: String::new(),
            error: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AddField {
    Type,
    Title,
    Rating,
    Status,
}

pub struct AddModal {
    pub type_idx: usize,
    pub title: String,
    pub rating: String,
    pub status_idx: usize,
    pub focus: AddField,
}

impl Default for AddModal {
    fn default() -> Self {
        Self::new()
    }
}

impl AddModal {
    pub fn new() -> Self {
        let status_idx = STATUSES.iter().position(|s| *s == STATUS_BACKLOG).unwrap_or(0);
        Self {
            type_idx: 0,
            title: String::new(),
            rating: String::new(),
            status_idx,
            focus: AddField::Type,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    Type,
    Title,
    Status,
}

pub struct EditModal {
    pub row_id: Uuid,
    pub type_idx: usize,
    pub title: String,
    pub status_idx: usize,
    pub focus: EditField,
    pub display_elo: f64,
    pub display_matches: u32,
}

impl EditModal {
    pub fn new(r: &Row, types: &[String]) -> Self {
        let type_idx = types.iter().position(|t| t == &r.type_).unwrap_or(0);
        let status_idx = STATUSES.iter().position(|s| *s == r.status).unwrap_or(0);
        Self {
            row_id: r.id,
            type_idx,
            title: r.title.clone(),
            status_idx,
            focus: EditField::Title,
            display_elo: r.elo,
            display_matches: r.matches,
        }
    }
}

pub enum ConfirmAction {
    DeleteRow(Uuid),
}

pub struct ConfirmModal {
    pub message: String,
    pub action: ConfirmAction,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum SortMode {
    #[default]
    Default,
    Elo,
    Matches,
    DateAdded,
    Title,
}

impl SortMode {
    pub fn label(self) -> &'static str {
        match self {
            SortMode::Default => "default",
            SortMode::Elo => "elo desc",
            SortMode::Matches => "matches desc",
            SortMode::DateAdded => "date desc",
            SortMode::Title => "title",
        }
    }

    pub fn next(self) -> Self {
        match self {
            SortMode::Default => SortMode::Elo,
            SortMode::Elo => SortMode::Matches,
            SortMode::Matches => SortMode::DateAdded,
            SortMode::DateAdded => SortMode::Title,
            SortMode::Title => SortMode::Default,
        }
    }
}

#[derive(Default)]
pub struct ListState {
    pub visible: Vec<usize>,
    pub cursor: usize,
    pub search_query: String,
    pub search_active: bool,
    pub pending_only: bool,
    pub type_filter: Option<String>,
    pub sort: SortMode,
}

#[derive(Default)]
pub struct CompareState {
    pub current_pair: Option<(Uuid, Uuid)>,
    pub last_result: Option<VoteResult>,
}

pub struct VoteResult {
    pub winner_title: String,
    pub loser_title: String,
    pub delta_w: f64,
    pub delta_l: f64,
}

pub struct UndoEntry {
    pub a_id: Uuid,
    pub b_id: Uuid,
    pub old_elo_a: f64,
    pub old_elo_b: f64,
    pub old_matches_a: u32,
    pub old_matches_b: u32,
}

pub struct App {
    pub api: Api,
    pub pairer: Pairer,
    pub rows: Vec<Row>,
    pub types: Vec<String>,
    pub screen: Screen,
    pub modal: Modal,
    pub list: ListState,
    pub compare: CompareState,
    pub undo_stack: Vec<UndoEntry>,
    pub show_help: bool,
    pub stats_scroll: u16,
    pub stats_content_height: std::cell::Cell<u16>,
    pub last_error: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(api: Api, pairer: Pairer) -> Result<Self> {
        let rows = api.list_rows()?;
        let types = api.list_types()?;
        let mut app = Self {
            api,
            pairer,
            rows,
            types,
            screen: Screen::List,
            modal: Modal::None,
            list: ListState::default(),
            compare: CompareState::default(),
            undo_stack: Vec::new(),
            show_help: false,
            stats_scroll: 0,
            stats_content_height: std::cell::Cell::new(0),
            last_error: None,
            should_quit: false,
        };
        app.refresh_list();
        Ok(app)
    }

    pub fn quit_app(&mut self) {
        self.should_quit = true;
    }

    fn record<T>(&mut self, label: &str, result: Result<T>) -> Option<T> {
        match result {
            Ok(v) => {
                self.last_error = None;
                Some(v)
            }
            Err(e) => {
                self.last_error = Some(format!("{label}: {e}"));
                None
            }
        }
    }

    fn replace_row(&mut self, updated: Row) {
        if let Some(slot) = self.rows.iter_mut().find(|r| r.id == updated.id) {
            *slot = updated;
        }
    }

    fn row_idx_by_id(&self, id: Uuid) -> Option<usize> {
        self.rows.iter().position(|r| r.id == id)
    }

    // ----------------------------
    // List: filtering + sorting
    // ----------------------------
    pub fn refresh_list(&mut self) {
        let q = self.list.search_query.to_lowercase();
        let mut visible: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                if let Some(t) = &self.list.type_filter {
                    if &r.type_ != t {
                        return false;
                    }
                }
                if self.list.pending_only && r.status == STATUS_DONE {
                    return false;
                }
                if !q.is_empty()
                    && !r.title.to_lowercase().contains(&q)
                    && !r.type_.to_lowercase().contains(&q)
                {
                    return false;
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        let status_rank = |s: &str| {
            STATUSES
                .iter()
                .position(|x| *x == s)
                .unwrap_or(STATUSES.len())
        };
        match self.list.sort {
            SortMode::Default => {
                visible.sort_by(|&a, &b| {
                    let ra = &self.rows[a];
                    let rb = &self.rows[b];
                    let sa = status_rank(&ra.status);
                    let sb = status_rank(&rb.status);
                    if sa != sb {
                        return sa.cmp(&sb);
                    }
                    if is_rankable(&ra.status) {
                        rb.elo
                            .partial_cmp(&ra.elo)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        ra.title.to_lowercase().cmp(&rb.title.to_lowercase())
                    }
                });
            }
            SortMode::Elo => {
                visible.sort_by(|&a, &b| {
                    self.rows[b]
                        .elo
                        .partial_cmp(&self.rows[a].elo)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortMode::Matches => {
                visible.sort_by(|&a, &b| {
                    self.rows[b].matches.cmp(&self.rows[a].matches).then_with(|| {
                        self.rows[a]
                            .title
                            .to_lowercase()
                            .cmp(&self.rows[b].title.to_lowercase())
                    })
                });
            }
            SortMode::DateAdded => {
                visible.sort_by(|&a, &b| {
                    self.rows[b]
                        .date_added
                        .cmp(&self.rows[a].date_added)
                        .then_with(|| {
                            self.rows[a]
                                .title
                                .to_lowercase()
                                .cmp(&self.rows[b].title.to_lowercase())
                        })
                });
            }
            SortMode::Title => {
                visible.sort_by(|&a, &b| {
                    self.rows[a]
                        .title
                        .to_lowercase()
                        .cmp(&self.rows[b].title.to_lowercase())
                });
            }
        }

        self.list.visible = visible;
        if self.list.cursor >= self.list.visible.len() {
            self.list.cursor = self.list.visible.len().saturating_sub(1);
        }
    }

    fn current_row_idx(&self) -> Option<usize> {
        self.list.visible.get(self.list.cursor).copied()
    }

    pub fn available_types(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for t in &self.types {
            if !seen.contains(t) && self.rows.iter().any(|r| &r.type_ == t) {
                seen.insert(t.clone());
                out.push(t.clone());
            }
        }
        // Orphaned types still appear (rows reference a type that's been removed).
        for r in &self.rows {
            if !seen.contains(&r.type_) {
                seen.insert(r.type_.clone());
                out.push(r.type_.clone());
            }
        }
        out
    }

    pub fn cycle_type(&mut self, forward: bool) {
        let types = self.available_types();
        if types.is_empty() {
            return;
        }
        self.list.type_filter = match &self.list.type_filter {
            None => Some(if forward {
                types[0].clone()
            } else {
                types[types.len() - 1].clone()
            }),
            Some(cur) => {
                let idx = types.iter().position(|t| t == cur);
                match (idx, forward) {
                    (None, true) => Some(types[0].clone()),
                    (None, false) => Some(types[types.len() - 1].clone()),
                    (Some(i), true) => {
                        if i + 1 >= types.len() {
                            None
                        } else {
                            Some(types[i + 1].clone())
                        }
                    }
                    (Some(i), false) => {
                        if i == 0 {
                            None
                        } else {
                            Some(types[i - 1].clone())
                        }
                    }
                }
            }
        };
        self.refresh_list();
    }

    pub fn toggle_pending(&mut self) {
        self.list.pending_only = !self.list.pending_only;
        self.refresh_list();
    }

    pub fn cycle_sort(&mut self) {
        self.list.sort = self.list.sort.next();
        self.refresh_list();
    }

    pub fn cursor_down(&mut self) {
        if self.list.cursor + 1 < self.list.visible.len() {
            self.list.cursor += 1;
        }
    }

    pub fn cursor_up(&mut self) {
        if self.list.cursor > 0 {
            self.list.cursor -= 1;
        }
    }

    pub fn cursor_home(&mut self) {
        self.list.cursor = 0;
    }

    pub fn cursor_end(&mut self) {
        if !self.list.visible.is_empty() {
            self.list.cursor = self.list.visible.len() - 1;
        }
    }

    pub fn yank(&mut self) {
        let Some(idx) = self.current_row_idx() else {
            return;
        };
        let title = self.rows[idx].title.clone();
        if let Ok(mut child) = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(title.as_bytes());
            }
            let _ = child.wait();
        }
    }

    pub fn toggle_status_at_cursor(&mut self) {
        let Some(idx) = self.current_row_idx() else {
            return;
        };
        let id = self.rows[idx].id;
        let cur = STATUSES
            .iter()
            .position(|s| *s == self.rows[idx].status)
            .unwrap_or(0);
        let next = STATUSES[(cur + 1) % STATUSES.len()].to_string();
        let updated = self.api.set_status(id, &next);
        if let Some(row) = self.record("set_status", updated) {
            self.replace_row(row);
            self.refresh_list();
        }
    }

    pub fn begin_delete(&mut self) {
        let Some(idx) = self.current_row_idx() else {
            return;
        };
        let id = self.rows[idx].id;
        let title = self.rows[idx].title.clone();
        self.modal = Modal::Confirm(ConfirmModal {
            message: format!("Delete {title}?"),
            action: ConfirmAction::DeleteRow(id),
        });
    }

    pub fn begin_edit(&mut self) {
        let Some(idx) = self.current_row_idx() else {
            return;
        };
        self.modal = Modal::Edit(EditModal::new(&self.rows[idx], &self.types));
    }

    pub fn begin_add(&mut self) {
        self.modal = Modal::Add(AddModal::new());
    }

    pub fn begin_type_manager(&mut self) {
        self.modal = Modal::TypeManager(TypeManagerModal::default());
    }

    pub fn type_mgr_cursor_down(&mut self) {
        let Modal::TypeManager(m) = &mut self.modal else { return };
        if m.cursor + 1 < self.types.len() {
            m.cursor += 1;
        }
    }

    pub fn type_mgr_cursor_up(&mut self) {
        let Modal::TypeManager(m) = &mut self.modal else { return };
        if m.cursor > 0 {
            m.cursor -= 1;
        }
    }

    pub fn type_mgr_cursor_home(&mut self) {
        let Modal::TypeManager(m) = &mut self.modal else { return };
        m.cursor = 0;
    }

    pub fn type_mgr_cursor_end(&mut self) {
        let Modal::TypeManager(m) = &mut self.modal else { return };
        m.cursor = self.types.len().saturating_sub(1);
    }

    pub fn type_mgr_begin_add(&mut self) {
        let Modal::TypeManager(m) = &mut self.modal else { return };
        m.mode = TypeManagerMode::AddInput;
        m.buffer.clear();
        m.error = None;
    }

    pub fn type_mgr_begin_rename(&mut self) {
        let Modal::TypeManager(m) = &mut self.modal else { return };
        let Some(name) = self.types.get(m.cursor).cloned() else { return };
        m.mode = TypeManagerMode::RenameInput;
        m.buffer = name;
        m.error = None;
    }

    pub fn type_mgr_input_cancel(&mut self) {
        let Modal::TypeManager(m) = &mut self.modal else { return };
        m.mode = TypeManagerMode::Browse;
        m.buffer.clear();
        m.error = None;
    }

    pub fn type_mgr_input_push(&mut self, c: char) {
        let Modal::TypeManager(m) = &mut self.modal else { return };
        if m.buffer.chars().count() < MAX_TYPE_LEN {
            m.buffer.push(c);
        }
    }

    pub fn type_mgr_input_pop(&mut self) {
        let Modal::TypeManager(m) = &mut self.modal else { return };
        m.buffer.pop();
    }

    pub fn type_mgr_submit_input(&mut self) {
        let Modal::TypeManager(m) = &self.modal else { return };
        match m.mode {
            TypeManagerMode::AddInput => self.type_mgr_submit_add(),
            TypeManagerMode::RenameInput => self.type_mgr_submit_rename(),
            TypeManagerMode::Browse => {}
        }
    }

    fn type_mgr_submit_add(&mut self) {
        let name = {
            let Modal::TypeManager(m) = &self.modal else { return };
            m.buffer.trim().to_string()
        };
        if let Some(err) = validate_type_name(&name) {
            self.type_mgr_set_error(err);
            return;
        }
        if self.types.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
            self.type_mgr_set_error("type already exists".to_string());
            return;
        }
        match self.api.add_type(&name) {
            Ok(list) => {
                self.types = list;
                let new_idx = self
                    .types
                    .iter()
                    .position(|t| t.eq_ignore_ascii_case(&name));
                if let Modal::TypeManager(m) = &mut self.modal {
                    m.mode = TypeManagerMode::Browse;
                    m.buffer.clear();
                    m.error = None;
                    if let Some(idx) = new_idx {
                        m.cursor = idx;
                    }
                }
            }
            Err(e) => self.type_mgr_set_error(e.to_string()),
        }
    }

    fn type_mgr_submit_rename(&mut self) {
        let (cursor, new_name) = {
            let Modal::TypeManager(m) = &self.modal else { return };
            (m.cursor, m.buffer.trim().to_string())
        };
        if let Some(err) = validate_type_name(&new_name) {
            self.type_mgr_set_error(err);
            return;
        }
        let Some(old) = self.types.get(cursor).cloned() else { return };
        if old == new_name {
            if let Modal::TypeManager(m) = &mut self.modal {
                m.mode = TypeManagerMode::Browse;
                m.buffer.clear();
                m.error = None;
            }
            return;
        }
        let case_only = old.eq_ignore_ascii_case(&new_name);
        if !case_only
            && self.types.iter().any(|t| t.eq_ignore_ascii_case(&new_name))
        {
            self.type_mgr_set_error("target name already exists".to_string());
            return;
        }
        match self.api.rename_type(&old, &new_name) {
            Ok(list) => {
                self.types = list;
                // Rename cascades to rows.type server-side; refresh local cache.
                if let Ok(rows) = self.api.list_rows() {
                    self.rows = rows;
                    self.refresh_list();
                }
                let new_idx = self
                    .types
                    .iter()
                    .position(|t| t.eq_ignore_ascii_case(&new_name));
                if let Modal::TypeManager(m) = &mut self.modal {
                    m.mode = TypeManagerMode::Browse;
                    m.buffer.clear();
                    m.error = None;
                    if let Some(idx) = new_idx {
                        m.cursor = idx;
                    }
                }
            }
            Err(e) => self.type_mgr_set_error(e.to_string()),
        }
    }

    pub fn type_mgr_delete(&mut self) {
        let Some(name) = ({
            let Modal::TypeManager(m) = &self.modal else { return };
            self.types.get(m.cursor).cloned()
        }) else {
            return;
        };
        if self.rows.iter().any(|r| r.type_.eq_ignore_ascii_case(&name)) {
            self.type_mgr_set_error(format!("'{name}' is in use"));
            return;
        }
        match self.api.delete_type(&name) {
            Ok(()) => {
                self.types.retain(|t| !t.eq_ignore_ascii_case(&name));
                if let Modal::TypeManager(m) = &mut self.modal {
                    if m.cursor >= self.types.len() {
                        m.cursor = self.types.len().saturating_sub(1);
                    }
                    m.error = None;
                }
            }
            Err(e) => self.type_mgr_set_error(e.to_string()),
        }
    }

    pub fn type_mgr_move(&mut self, delta: i32) {
        let cursor = {
            let Modal::TypeManager(m) = &self.modal else { return };
            m.cursor
        };
        if self.types.is_empty() {
            return;
        }
        let len = self.types.len();
        let new_idx = (cursor as i32 + delta).clamp(0, (len as i32) - 1) as usize;
        if new_idx == cursor {
            return;
        }
        let mut reordered = self.types.clone();
        reordered.swap(cursor, new_idx);
        match self.api.reorder_types(&reordered) {
            Ok(list) => {
                self.types = list;
                if let Modal::TypeManager(m) = &mut self.modal {
                    m.cursor = new_idx;
                    m.error = None;
                }
            }
            Err(e) => self.type_mgr_set_error(e.to_string()),
        }
    }

    fn type_mgr_set_error(&mut self, err: String) {
        if let Modal::TypeManager(m) = &mut self.modal {
            m.error = Some(err);
        }
    }

    pub fn open_search(&mut self) {
        self.list.search_active = true;
        self.list.search_query.clear();
        self.refresh_list();
    }

    pub fn close_search(&mut self, clear: bool) {
        self.list.search_active = false;
        if clear {
            self.list.search_query.clear();
            self.refresh_list();
        }
    }

    pub fn search_push(&mut self, c: char) {
        self.list.search_query.push(c);
        self.refresh_list();
    }

    pub fn search_pop(&mut self) {
        self.list.search_query.pop();
        self.refresh_list();
    }

    pub fn reload(&mut self) {
        let rows = self.api.list_rows();
        if let Some(rows) = self.record("reload", rows) {
            self.rows = rows;
            self.refresh_list();
        }
    }

    pub fn switch_screen(&mut self, screen: Screen) {
        self.screen = screen;
        if screen == Screen::Compare {
            self.next_pair();
        }
    }

    pub fn toggle_stats(&mut self) {
        self.screen = match self.screen {
            Screen::Stats => Screen::List,
            _ => {
                self.stats_scroll = 0;
                Screen::Stats
            }
        };
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn stats_scroll_by(&mut self, delta: i16) {
        let new = self.stats_scroll as i32 + delta as i32;
        self.stats_scroll = new.max(0) as u16;
    }

    pub fn stats_scroll_top(&mut self) {
        self.stats_scroll = 0;
    }

    pub fn stats_scroll_bottom(&mut self) {
        self.stats_scroll = self.stats_content_height.get();
    }

    // ----------------------------
    // Compare
    // ----------------------------
    fn eligible_indices(&self) -> Vec<usize> {
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                if !is_rankable(&r.status) {
                    return false;
                }
                if let Some(t) = &self.list.type_filter {
                    if &r.type_ != t {
                        return false;
                    }
                }
                true
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn next_pair(&mut self) {
        let eligible = self.eligible_indices();
        self.compare.current_pair = self.pairer.pick(&self.rows, &eligible);
    }

    pub fn vote(&mut self, a_wins: bool) {
        let Some((ai, bi)) = self.compare.current_pair else {
            return;
        };
        // Resolve cached snapshots before mutating.
        let (Some(a_idx), Some(b_idx)) = (self.row_idx_by_id(ai), self.row_idx_by_id(bi)) else {
            self.compare.current_pair = None;
            return;
        };
        let (winner_id, loser_id) = if a_wins { (ai, bi) } else { (bi, ai) };
        let snapshot = UndoEntry {
            a_id: ai,
            b_id: bi,
            old_elo_a: self.rows[a_idx].elo,
            old_elo_b: self.rows[b_idx].elo,
            old_matches_a: self.rows[a_idx].matches,
            old_matches_b: self.rows[b_idx].matches,
        };

        let resp = self.api.vote(winner_id, loser_id);
        let Some(resp) = self.record("vote", resp) else {
            return;
        };

        let result = VoteResult {
            winner_title: resp.winner.title.clone(),
            loser_title: resp.loser.title.clone(),
            delta_w: resp.delta_winner,
            delta_l: resp.delta_loser,
        };
        self.pairer.remember(&resp.winner, &resp.loser);
        self.replace_row(resp.winner);
        self.replace_row(resp.loser);
        self.undo_stack.push(snapshot);
        self.compare.last_result = Some(result);
        self.next_pair();
    }

    pub fn skip(&mut self) {
        self.compare.last_result = None;
        self.next_pair();
    }

    pub fn undo_vote(&mut self) {
        let Some(u) = self.undo_stack.pop() else {
            return;
        };
        let req = UndoRequest {
            a_id: u.a_id,
            b_id: u.b_id,
            old_elo_a: u.old_elo_a,
            old_elo_b: u.old_elo_b,
            old_matches_a: u.old_matches_a,
            old_matches_b: u.old_matches_b,
        };
        let ok = self.api.undo(&req);
        if self.record("undo", ok).is_none() {
            return;
        }
        if let Some(idx) = self.row_idx_by_id(u.a_id) {
            self.rows[idx].elo = u.old_elo_a;
            self.rows[idx].matches = u.old_matches_a;
        }
        if let Some(idx) = self.row_idx_by_id(u.b_id) {
            self.rows[idx].elo = u.old_elo_b;
            self.rows[idx].matches = u.old_matches_b;
        }
        self.pairer.forget_last();
        self.compare.current_pair = Some((u.a_id, u.b_id));
        self.compare.last_result = None;
    }

    // ----------------------------
    // Modal handlers
    // ----------------------------
    pub fn confirm_yes(&mut self) {
        if let Modal::Confirm(c) = std::mem::replace(&mut self.modal, Modal::None) {
            match c.action {
                ConfirmAction::DeleteRow(id) => {
                    let res = self.api.delete_row(id);
                    if self.record("delete", res).is_some() {
                        self.rows.retain(|r| r.id != id);
                        self.refresh_list();
                    }
                }
            }
        }
    }

    pub fn confirm_no(&mut self) {
        self.modal = Modal::None;
    }

    pub fn submit_add(&mut self) {
        let Modal::Add(m) = &self.modal else { return };
        let title = m.title.trim().to_string();
        if title.is_empty() {
            return;
        }
        let Some(type_) = self.types.get(m.type_idx).cloned() else {
            return;
        };
        let req = AddRequest {
            type_,
            title,
            rating: m.rating.trim().parse::<f64>().ok(),
            status: STATUSES[m.status_idx].to_string(),
        };
        let resp = self.api.add_row(&req);
        if let Some(row) = self.record("add", resp) {
            self.rows.push(row);
            self.modal = Modal::None;
            self.refresh_list();
        }
    }

    pub fn submit_edit(&mut self) {
        let Modal::Edit(m) = &self.modal else { return };
        let title = m.title.trim().to_string();
        if title.is_empty() {
            return;
        }
        let Some(type_) = self.types.get(m.type_idx).cloned() else {
            return;
        };
        let req = EditRequest {
            type_,
            title,
            status: STATUSES[m.status_idx].to_string(),
        };
        let id = m.row_id;
        let resp = self.api.edit_row(id, &req);
        if let Some(row) = self.record("edit", resp) {
            self.replace_row(row);
            self.modal = Modal::None;
            self.refresh_list();
        }
    }
}
