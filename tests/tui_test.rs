mod common;

use azdocs::labels::Labels;

use azdocs::store::Store;
use azdocs::tui::{App, Pane, Screen};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent};

fn seeded_app() -> (Store, App) {
    let store = Store::open_in_memory().unwrap();
    let id = common::seed_estate(&store);
    let mut app = App::new(store.list_snapshots().unwrap(), Labels::default().tui);
    app.load_estate(&store, &id).unwrap();
    (store, app)
}

fn buffer_text(app: &App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal
        .draw(|frame| azdocs::tui::render_for_test(frame, app))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn estate_view_shows_tree_resources_and_detail() {
    let (_store, app) = seeded_app();

    let text = buffer_text(&app);

    assert!(
        text.contains("Production") && text.contains("Resources") && text.contains("Detail"),
        "buffer: {text}"
    );
}

#[test]
fn snapshot_picker_lists_snapshot_counts() {
    let (store, _) = seeded_app();
    let app = App::new(store.list_snapshots().unwrap(), Labels::default().tui);

    let text = buffer_text(&app);

    assert!(
        text.contains("Snapshots") && text.contains("complete"),
        "buffer: {text}"
    );
}

#[test]
fn tree_selection_scopes_resource_list() {
    let (store, mut app) = seeded_app();

    // Tree is sorted by subscription name: Development first, then its rg-dev.
    app.handle_key(KeyEvent::from(KeyCode::Down), &store);
    let visible = app.visible_resources();

    assert!(
        !visible.is_empty()
            && visible
                .iter()
                .all(|r| r.resource_group.as_deref() == Some("rg-dev")),
        "visible: {:?}",
        visible.iter().map(|r| &r.name).collect::<Vec<_>>()
    );
}

#[test]
fn filter_narrows_resource_list() {
    let (store, mut app) = seeded_app();

    app.handle_key(KeyEvent::from(KeyCode::Char('/')), &store);
    for c in "web-d".chars() {
        app.handle_key(KeyEvent::from(KeyCode::Char(c)), &store);
    }
    app.handle_key(KeyEvent::from(KeyCode::Enter), &store);

    let names: Vec<&str> = app
        .visible_resources()
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(names, vec!["id-web-dev", "web-dev"]);
}

#[test]
fn findings_enter_jumps_to_resource_detail() {
    let (store, mut app) = seeded_app();

    app.handle_key(KeyEvent::from(KeyCode::Char('f')), &store);
    assert_eq!(app.screen, Screen::Findings);
    app.handle_key(KeyEvent::from(KeyCode::Enter), &store);

    assert_eq!(
        (app.screen, app.pane),
        (Screen::Estate, Pane::Detail),
        "selected: {:?}",
        app.selected_resource().map(|r| &r.name)
    );
}

#[test]
fn q_quits_from_any_screen() {
    let (store, mut app) = seeded_app();

    app.handle_key(KeyEvent::from(KeyCode::Char('q')), &store);

    assert!(app.quit);
}
