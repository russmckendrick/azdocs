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
    assert_eq!(
        names,
        vec!["web-dev-snap", "id-web-dev", "web-dev-cert", "web-dev"]
    );
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

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::from(code)
}

#[test]
fn unit_help_overlay_opens_on_question_mark_and_closes_on_escape() {
    let (store, mut app) = seeded_app();

    app.handle_key(key(KeyCode::Char('?')), &store);
    assert!(app.help);
    assert!(buffer_text(&app).contains("Keys (? or Esc closes)"));
    app.handle_key(key(KeyCode::Down), &store);
    assert_eq!(app.tree_index, 0, "keys are swallowed while help is open");

    app.handle_key(key(KeyCode::Esc), &store);
    assert!(!app.help);
}

#[test]
fn unit_tab_cycles_the_findings_severity_floor() {
    let (store, mut app) = seeded_app();
    app.handle_key(key(KeyCode::Char('f')), &store);
    let all = app.visible_findings().len();

    app.handle_key(key(KeyCode::Tab), &store);
    assert_eq!(app.severity_filter, Some(azdocs::model::Severity::High));
    let high = app.visible_findings().len();
    assert!(high > 0 && high < all, "all {all}, high {high}");
    assert!(buffer_text(&app).contains("severity ≥ high"));

    app.handle_key(key(KeyCode::Tab), &store);
    app.handle_key(key(KeyCode::Tab), &store);
    app.handle_key(key(KeyCode::Tab), &store);
    assert_eq!(app.severity_filter, None);
    assert_eq!(app.visible_findings().len(), all);
}

#[test]
fn unit_enter_in_the_detail_pane_follows_the_highlighted_relationship() {
    let (store, mut app) = seeded_app();
    // Down the tree to rg-app, into the list, find the VM, open its detail.
    for _ in 0..2 {
        app.handle_key(key(KeyCode::Down), &store);
    }
    app.handle_key(key(KeyCode::Enter), &store);
    let vm = app
        .visible_resources()
        .iter()
        .position(|r| r.name == "vm-app-01")
        .expect("vm in rg-app");
    for _ in 0..vm {
        app.handle_key(key(KeyCode::Down), &store);
    }
    app.handle_key(key(KeyCode::Enter), &store);
    assert_eq!(app.pane, Pane::Detail);
    let selected = app.selected_resource().unwrap().id.clone();
    let related: Vec<String> = app
        .related_edges()
        .iter()
        .map(|edge| edge.other_end(&selected).unwrap().to_owned())
        .collect();
    assert!(related.len() > 1, "the VM has several relationships");

    app.handle_key(key(KeyCode::Char('n')), &store);
    assert_eq!(app.related_index, 1);
    app.handle_key(key(KeyCode::Char('p')), &store);
    assert_eq!(app.related_index, 0);
    let target = related[0].clone();

    app.handle_key(key(KeyCode::Enter), &store);
    assert_eq!(app.selected_resource().unwrap().id, target);
    assert_eq!(app.pane, Pane::Detail);
}

#[test]
fn unit_snapshot_picker_shows_the_tenant_and_survives_short_ids() {
    let (store, _) = seeded_app();
    let mut entries = store.list_snapshots().unwrap();
    entries[0].snapshot.id = "ab".to_owned();
    entries[0].snapshot.tenant_id = "t".to_owned();
    let app = App::new(entries, Labels::default().tui);

    let text = buffer_text(&app);

    assert!(
        text.contains("ab ") && text.contains(" t "),
        "buffer: {text}"
    );
}

#[test]
fn unit_load_failure_is_said_in_the_footer_and_keeps_the_picker() {
    let (store, _) = seeded_app();
    let mut entries = store.list_snapshots().unwrap();
    entries[0].snapshot.id = "missing".to_owned();
    let mut app = App::new(entries, Labels::default().tui);

    app.handle_key(key(KeyCode::Enter), &store);

    assert_eq!(app.screen, Screen::Snapshots);
    let status = app.status.clone().expect("failure recorded");
    assert!(
        status.starts_with("Could not open snapshot missing"),
        "{status}"
    );
    assert!(buffer_text(&app).contains("Could not open snapshot"));
}
