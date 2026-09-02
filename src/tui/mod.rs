mod ui;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::error::StoreError;
use crate::labels::TuiLabels;
use crate::model::{Edge, Finding, Resource, ResourceGroup, Subscription};
use crate::store::{SnapshotCounts, Store};

/// Which screen is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Snapshots,
    Estate,
    Findings,
}

/// Which estate pane has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Tree,
    List,
    Detail,
}

/// One row in the subscription/resource-group tree.
#[derive(Debug)]
pub struct TreeEntry {
    pub label: String,
    pub subscription_id: String,
    /// None = whole subscription; Some = one resource group (lowercased).
    pub resource_group: Option<String>,
}

/// Everything loaded for one snapshot.
pub struct EstateData {
    pub snapshot_id: String,
    pub subscriptions: Vec<Subscription>,
    pub resource_groups: Vec<ResourceGroup>,
    pub resources: Vec<Resource>,
    pub findings: Vec<Finding>,
    pub edges: Vec<Edge>,
    pub tree: Vec<TreeEntry>,
}

/// Pure state machine over loaded data: `handle_key` mutates, `ui::render`
/// draws. No terminal access here, so tests drive it directly.
pub struct App {
    pub snapshots: Vec<SnapshotCounts>,
    pub screen: Screen,
    pub pane: Pane,
    pub estate: Option<EstateData>,
    pub snapshot_index: usize,
    pub tree_index: usize,
    pub list_index: usize,
    pub findings_index: usize,
    pub detail_scroll: u16,
    pub filter: String,
    pub filtering: bool,
    pub quit: bool,
    /// Every word the screens draw, resolved once at startup.
    pub labels: TuiLabels,
}

impl App {
    pub fn new(snapshots: Vec<SnapshotCounts>, labels: TuiLabels) -> Self {
        Self {
            snapshots,
            screen: Screen::Snapshots,
            pane: Pane::Tree,
            estate: None,
            snapshot_index: 0,
            tree_index: 0,
            list_index: 0,
            findings_index: 0,
            detail_scroll: 0,
            filter: String::new(),
            filtering: false,
            quit: false,
            labels,
        }
    }

    pub fn load_estate(&mut self, store: &Store, snapshot_id: &str) -> Result<(), StoreError> {
        let subscriptions = store.subscriptions(snapshot_id)?;
        let resource_groups = store.resource_groups(snapshot_id)?;
        let mut tree = Vec::new();
        for sub in &subscriptions {
            tree.push(TreeEntry {
                label: format!("▸ {}", sub.display_name),
                subscription_id: sub.subscription_id.clone(),
                resource_group: None,
            });
            for rg in resource_groups
                .iter()
                .filter(|rg| rg.subscription_id == sub.subscription_id)
            {
                tree.push(TreeEntry {
                    label: format!("    {}", rg.name),
                    subscription_id: sub.subscription_id.clone(),
                    resource_group: Some(rg.name.to_lowercase()),
                });
            }
        }
        self.estate = Some(EstateData {
            snapshot_id: snapshot_id.to_owned(),
            subscriptions,
            resource_groups,
            resources: store.resources(snapshot_id)?,
            findings: store.findings(snapshot_id)?,
            edges: store.edges(snapshot_id)?,
            tree,
        });
        self.screen = Screen::Estate;
        self.pane = Pane::Tree;
        self.tree_index = 0;
        self.list_index = 0;
        Ok(())
    }

    /// Resources matching the current tree selection and filter text.
    pub fn visible_resources(&self) -> Vec<&Resource> {
        let Some(estate) = &self.estate else {
            return Vec::new();
        };
        let entry = estate.tree.get(self.tree_index);
        let filter = self.filter.to_lowercase();
        estate
            .resources
            .iter()
            .filter(|r| {
                entry.is_none_or(|entry| {
                    r.subscription_id == entry.subscription_id
                        && entry
                            .resource_group
                            .as_ref()
                            .is_none_or(|rg| r.resource_group.as_deref() == Some(rg.as_str()))
                })
            })
            .filter(|r| {
                filter.is_empty()
                    || r.name.to_lowercase().contains(&filter)
                    || r.azure_type.contains(&filter)
                    || r.tags
                        .as_ref()
                        .is_some_and(|t| t.to_string().to_lowercase().contains(&filter))
            })
            .collect()
    }

    pub fn selected_resource(&self) -> Option<&Resource> {
        self.visible_resources().into_iter().nth(self.list_index)
    }

    pub fn handle_key(&mut self, key: KeyEvent, store: &Store) {
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    self.filtering = false;
                    self.filter.clear();
                }
                KeyCode::Enter => self.filtering = false,
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    self.list_index = 0;
                }
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.quit = true;
            }
            _ => match self.screen {
                Screen::Snapshots => self.handle_snapshots_key(key, store),
                Screen::Estate => self.handle_estate_key(key),
                Screen::Findings => self.handle_findings_key(key),
            },
        }
    }

    fn handle_snapshots_key(&mut self, key: KeyEvent, store: &Store) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.snapshot_index =
                    (self.snapshot_index + 1).min(self.snapshots.len().saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.snapshot_index = self.snapshot_index.saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(entry) = self.snapshots.get(self.snapshot_index) {
                    let id = entry.snapshot.id.clone();
                    // Load failures leave the picker on screen; nothing to lose.
                    let _ = self.load_estate(store, &id);
                }
            }
            _ => {}
        }
    }

    fn handle_estate_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.pane = match self.pane {
                    Pane::Tree => Pane::List,
                    Pane::List => Pane::Detail,
                    Pane::Detail => Pane::Tree,
                };
            }
            KeyCode::Char('/') => {
                self.filtering = true;
                self.filter.clear();
                self.pane = Pane::List;
            }
            KeyCode::Char('f') => {
                self.screen = Screen::Findings;
            }
            KeyCode::Char('s') | KeyCode::Esc if self.pane == Pane::Tree => {
                self.screen = Screen::Snapshots;
            }
            KeyCode::Down | KeyCode::Char('j') => match self.pane {
                Pane::Tree => {
                    let max = self
                        .estate
                        .as_ref()
                        .map_or(0, |e| e.tree.len().saturating_sub(1));
                    self.tree_index = (self.tree_index + 1).min(max);
                    self.list_index = 0;
                }
                Pane::List => {
                    let max = self.visible_resources().len().saturating_sub(1);
                    self.list_index = (self.list_index + 1).min(max);
                    self.detail_scroll = 0;
                }
                Pane::Detail => self.detail_scroll = self.detail_scroll.saturating_add(1),
            },
            KeyCode::Up | KeyCode::Char('k') => match self.pane {
                Pane::Tree => {
                    self.tree_index = self.tree_index.saturating_sub(1);
                    self.list_index = 0;
                }
                Pane::List => {
                    self.list_index = self.list_index.saturating_sub(1);
                    self.detail_scroll = 0;
                }
                Pane::Detail => self.detail_scroll = self.detail_scroll.saturating_sub(1),
            },
            KeyCode::Enter if self.pane == Pane::Tree => self.pane = Pane::List,
            KeyCode::Enter if self.pane == Pane::List => self.pane = Pane::Detail,
            KeyCode::Esc => self.pane = Pane::Tree,
            _ => {}
        }
    }

    fn handle_findings_key(&mut self, key: KeyEvent) {
        let count = self.estate.as_ref().map_or(0, |e| e.findings.len());
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.findings_index = (self.findings_index + 1).min(count.saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.findings_index = self.findings_index.saturating_sub(1);
            }
            KeyCode::Esc | KeyCode::Char('f') => self.screen = Screen::Estate,
            KeyCode::Enter => {
                // Jump to the finding's resource in the estate view.
                let target = self.estate.as_ref().and_then(|e| {
                    e.findings
                        .get(self.findings_index)
                        .and_then(|f| f.resource_id.clone())
                });
                if let Some(resource_id) = target {
                    self.jump_to_resource(&resource_id);
                }
            }
            _ => {}
        }
    }

    fn jump_to_resource(&mut self, resource_id: &str) {
        let Some(estate) = &self.estate else {
            return;
        };
        let Some(resource) = estate.resources.iter().find(|r| r.id == resource_id) else {
            return;
        };
        let tree_index = estate.tree.iter().position(|entry| {
            entry.subscription_id == resource.subscription_id
                && entry.resource_group.as_deref() == resource.resource_group.as_deref()
        });
        if let Some(tree_index) = tree_index {
            self.tree_index = tree_index;
            self.filter.clear();
            let position = self
                .visible_resources()
                .iter()
                .position(|r| r.id == resource_id);
            if let Some(position) = position {
                self.list_index = position;
                self.screen = Screen::Estate;
                self.pane = Pane::Detail;
                self.detail_scroll = 0;
            }
        }
    }
}

/// Test hook: render without a real terminal.
pub fn render_for_test(frame: &mut ratatui::Frame<'_>, app: &App) {
    ui::render(frame, app);
}

/// Interactive entry point: full-screen terminal until the user quits.
pub fn run(store: &Store, snapshot: &str, labels: TuiLabels) -> anyhow::Result<()> {
    let snapshots = store.list_snapshots()?;
    if snapshots.is_empty() {
        anyhow::bail!("no snapshots stored yet — run `azdocs collect` first");
    }
    let mut app = App::new(snapshots, labels);
    if let Ok(id) = store.resolve_snapshot(snapshot) {
        app.load_estate(store, &id)?;
    }

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app, store);
    ratatui::restore();
    result
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    store: &Store,
) -> anyhow::Result<()> {
    while !app.quit {
        terminal.draw(|frame| ui::render(frame, app))?;
        if let Event::Key(key) = event::read()? {
            app.handle_key(key, store);
        }
    }
    Ok(())
}
