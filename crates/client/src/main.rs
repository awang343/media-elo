mod api;
mod app;
mod pairer;
mod ui;

use crate::api::Api;
use crate::app::{AddField, App, EditField, Modal, Screen, TypeManagerMode};
use crate::pairer::Pairer;
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use media_elo_core::STATUSES;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

fn main() -> Result<()> {
    let api = Api::from_env()?;
    let pairer = Pairer::new();
    let mut app = App::new(api, pairer)?;

    let mut terminal = setup_terminal()?;
    let res = run_loop(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    res
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;
        if app.should_quit {
            return Ok(());
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => dispatch_key(app, key),
            _ => {}
        }
    }
}

fn dispatch_key(app: &mut App, key: KeyEvent) {
    match &app.modal {
        Modal::None => match app.screen {
            Screen::List => {
                if app.list.search_active {
                    handle_search_key(app, key);
                } else {
                    handle_list_key(app, key);
                }
            }
            Screen::Compare => handle_compare_key(app, key),
            Screen::Stats => handle_stats_key(app, key),
        },
        Modal::Add(_) => handle_add_key(app, key),
        Modal::Edit(_) => handle_edit_key(app, key),
        Modal::Confirm(_) => handle_confirm_key(app, key),
        Modal::TypeManager(_) => handle_type_manager_key(app, key),
    }
}

fn handle_list_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => app.switch_screen(Screen::Compare),
        KeyCode::Char('j') | KeyCode::Down => app.cursor_down(),
        KeyCode::Char('k') | KeyCode::Up => app.cursor_up(),
        KeyCode::Char('g') => app.cursor_home(),
        KeyCode::Char('G') => app.cursor_end(),
        KeyCode::Char('/') => app.open_search(),
        KeyCode::Char('p') => app.toggle_pending(),
        KeyCode::Char('t') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.cycle_type(false);
            } else {
                app.cycle_type(true);
            }
        }
        KeyCode::Char('T') => app.cycle_type(false),
        KeyCode::Char('N') => app.begin_type_manager(),
        KeyCode::Char('a') => app.begin_add(),
        KeyCode::Char('d') => app.toggle_status_at_cursor(),
        KeyCode::Char('x') => app.begin_delete(),
        KeyCode::Char('y') => app.yank(),
        KeyCode::Char('o') => app.cycle_sort(),
        KeyCode::Char('?') => app.toggle_stats(),
        KeyCode::Char('H') => app.toggle_help(),
        KeyCode::Char('r') => app.reload(),
        KeyCode::Char('q') => app.quit_app(),
        KeyCode::Enter => app.begin_edit(),
        _ => {}
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.close_search(true),
        KeyCode::Enter => app.close_search(false),
        KeyCode::Backspace => app.search_pop(),
        KeyCode::Char(c) => app.search_push(c),
        _ => {}
    }
}

fn handle_compare_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => app.switch_screen(Screen::List),
        KeyCode::Char('1') => app.vote(true),
        KeyCode::Char('2') => app.vote(false),
        KeyCode::Char('s') => app.skip(),
        KeyCode::Char('u') => app.undo_vote(),
        KeyCode::Char('H') => app.toggle_help(),
        KeyCode::Char('q') => app.quit_app(),
        _ => {}
    }
}

fn handle_stats_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc | KeyCode::Tab => {
            app.toggle_stats();
        }
        KeyCode::Char('H') => app.toggle_help(),
        KeyCode::Char('j') | KeyCode::Down => app.stats_scroll_by(1),
        KeyCode::Char('k') | KeyCode::Up => app.stats_scroll_by(-1),
        KeyCode::Char('d') | KeyCode::PageDown => app.stats_scroll_by(10),
        KeyCode::Char('u') | KeyCode::PageUp => app.stats_scroll_by(-10),
        KeyCode::Char('g') | KeyCode::Home => app.stats_scroll_top(),
        KeyCode::Char('G') | KeyCode::End => app.stats_scroll_bottom(),
        _ => {}
    }
}

fn handle_confirm_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') => app.confirm_yes(),
        KeyCode::Char('n') | KeyCode::Esc => app.confirm_no(),
        _ => {}
    }
}

fn handle_add_key(app: &mut App, key: KeyEvent) {
    let types_len = app.types.len().max(1);
    let Modal::Add(m) = &mut app.modal else { return };

    match key.code {
        KeyCode::Esc => {
            app.modal = Modal::None;
            return;
        }
        KeyCode::Enter => {
            match m.focus {
                AddField::Type => {
                    m.focus = AddField::Title;
                    return;
                }
                AddField::Title => {
                    m.focus = AddField::Rating;
                    return;
                }
                AddField::Rating => {
                    m.focus = AddField::Status;
                    return;
                }
                AddField::Status => {}
            }
            app.submit_add();
            return;
        }
        KeyCode::Tab => {
            m.focus = match m.focus {
                AddField::Type => AddField::Title,
                AddField::Title => AddField::Rating,
                AddField::Rating => AddField::Status,
                AddField::Status => AddField::Type,
            };
            return;
        }
        KeyCode::BackTab => {
            m.focus = match m.focus {
                AddField::Type => AddField::Status,
                AddField::Title => AddField::Type,
                AddField::Rating => AddField::Title,
                AddField::Status => AddField::Rating,
            };
            return;
        }
        _ => {}
    }

    match m.focus {
        AddField::Type => match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                m.type_idx = (m.type_idx + types_len - 1) % types_len;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                m.type_idx = (m.type_idx + 1) % types_len;
            }
            _ => {}
        },
        AddField::Status => match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                m.status_idx = (m.status_idx + STATUSES.len() - 1) % STATUSES.len();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                m.status_idx = (m.status_idx + 1) % STATUSES.len();
            }
            _ => {}
        },
        AddField::Title => handle_text_input(&mut m.title, key),
        AddField::Rating => handle_text_input(&mut m.rating, key),
    }
}

fn handle_edit_key(app: &mut App, key: KeyEvent) {
    let types_len = app.types.len().max(1);
    let Modal::Edit(m) = &mut app.modal else { return };

    match key.code {
        KeyCode::Esc => {
            app.modal = Modal::None;
            return;
        }
        KeyCode::Enter => {
            match m.focus {
                EditField::Type => {
                    m.focus = EditField::Title;
                    return;
                }
                EditField::Title => {
                    m.focus = EditField::Status;
                    return;
                }
                EditField::Status => {}
            }
            app.submit_edit();
            return;
        }
        KeyCode::Tab => {
            m.focus = match m.focus {
                EditField::Type => EditField::Title,
                EditField::Title => EditField::Status,
                EditField::Status => EditField::Type,
            };
            return;
        }
        KeyCode::BackTab => {
            m.focus = match m.focus {
                EditField::Type => EditField::Status,
                EditField::Title => EditField::Type,
                EditField::Status => EditField::Title,
            };
            return;
        }
        _ => {}
    }

    match m.focus {
        EditField::Type => match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                m.type_idx = (m.type_idx + types_len - 1) % types_len;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                m.type_idx = (m.type_idx + 1) % types_len;
            }
            _ => {}
        },
        EditField::Status => match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                m.status_idx = (m.status_idx + STATUSES.len() - 1) % STATUSES.len();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                m.status_idx = (m.status_idx + 1) % STATUSES.len();
            }
            _ => {}
        },
        EditField::Title => handle_text_input(&mut m.title, key),
    }
}

fn handle_type_manager_key(app: &mut App, key: KeyEvent) {
    let mode = match &app.modal {
        Modal::TypeManager(m) => m.mode,
        _ => return,
    };
    match mode {
        TypeManagerMode::Browse => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => app.modal = Modal::None,
            KeyCode::Char('j') | KeyCode::Down => app.type_mgr_cursor_down(),
            KeyCode::Char('k') | KeyCode::Up => app.type_mgr_cursor_up(),
            KeyCode::Char('g') => app.type_mgr_cursor_home(),
            KeyCode::Char('G') => app.type_mgr_cursor_end(),
            KeyCode::Char('a') => app.type_mgr_begin_add(),
            KeyCode::Char('r') => app.type_mgr_begin_rename(),
            KeyCode::Char('x') => app.type_mgr_delete(),
            KeyCode::Char('J') => app.type_mgr_move(1),
            KeyCode::Char('K') => app.type_mgr_move(-1),
            _ => {}
        },
        TypeManagerMode::AddInput | TypeManagerMode::RenameInput => match key.code {
            KeyCode::Esc => app.type_mgr_input_cancel(),
            KeyCode::Enter => app.type_mgr_submit_input(),
            KeyCode::Backspace => app.type_mgr_input_pop(),
            KeyCode::Char(c) => app.type_mgr_input_push(c),
            _ => {}
        },
    }
}

fn handle_text_input(buf: &mut String, key: KeyEvent) {
    match key.code {
        KeyCode::Backspace => {
            buf.pop();
        }
        KeyCode::Char(c) => {
            buf.push(c);
        }
        _ => {}
    }
}
