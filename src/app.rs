use anyhow::Result;
use ratatui::widgets::{ListState, TableState};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use crate::actions::{Action, ACTIONS_DIR, ACTIONS_FILE, FFMPEG_SUBACTIONS, copy_to_clipboard};
use crate::zones::{Zone, load_config};

const TOAST_DURATION: Duration = Duration::from_secs(2);

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn fd_binary() -> &'static str {
    static FD_BIN: LazyLock<&'static str> = LazyLock::new(|| {
        if Command::new("sh").args(["-c", "command -v fdfind"])
            .output().map(|o| o.status.success()).unwrap_or(false)
        {
            "fdfind"
        } else {
            "fd"
        }
    });
    *FD_BIN
}

fn preview_cmd() -> String {
    let script = dirs_next::config_dir()
        .unwrap_or_default()
        .join("fzf/preview.sh");
    // The entire value must be single-quoted for the outer sh -c shell;
    // {} inside is the fzf placeholder, not a shell expansion.
    if script.exists() {
        let escaped = script.to_string_lossy().replace('\'', "'\\''");
        format!("'{} {{}}'", escaped)
    } else {
        "'file {}; ls {} 2>/dev/null || true'".to_string()
    }
}

fn build_ffmpeg_command(path: &std::path::Path, key: char) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().to_string();
    match key {
        'e' => {
            let output = path.with_file_name(format!("{}-reenc.mp4", stem));
            Some(format!(
                "ffmpeg -i {} -c:v libx264 -crf 18 -preset slow -c:a copy {}",
                shell_escape(&path.to_string_lossy()),
                shell_escape(&output.to_string_lossy()),
            ))
        }
        'a' => {
            let output = path.with_file_name(format!("{}.mp3", stem));
            Some(format!(
                "ffmpeg -i {} -vn -c:a libmp3lame -q:a 2 {}",
                shell_escape(&path.to_string_lossy()),
                shell_escape(&output.to_string_lossy()),
            ))
        }
        _ => None,
    }
}

fn build_ffmpeg_preview(key: char) -> Option<&'static str> {
    match key {
        'e' => Some("ffmpeg -i {inputFile} -c:v libx264 -crf 18 -preset slow -c:a copy {outputFile}"),
        'a' => Some("ffmpeg -i {inputFile} -vn -c:a libmp3lame -q:a 2 {outputFile}"),
        _ => None,
    }
}

pub enum HistoryConfirm {
    DeleteEntry,
    ClearAll,
}


#[derive(PartialEq)]
pub enum Focus {
    Zones,
    History,
    Actions,
}

pub struct HistoryEntry {
    pub path: PathBuf,
    pub added_at: u64,
    pub zone_path: Option<String>,
    pub query: Option<String>,
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn history_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("easy-search_history")
}

fn load_history() -> Vec<HistoryEntry> {
    let path = history_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => content
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|l| {
                let parts: Vec<&str> = l.splitn(4, '\t').collect();
                match parts.len() {
                    4 => Some(HistoryEntry {
                        added_at: parts[0].parse().unwrap_or(0),
                        zone_path: if parts[1].is_empty() { None } else { Some(parts[1].to_string()) },
                        query: if parts[2].is_empty() { None } else { Some(parts[2].to_string()) },
                        path: PathBuf::from(parts[3]),
                    }),
                    2 => Some(HistoryEntry {
                        added_at: parts[0].parse().unwrap_or(0),
                        zone_path: None,
                        query: None,
                        path: PathBuf::from(parts[1]),
                    }),
                    _ => Some(HistoryEntry { path: PathBuf::from(l), added_at: 0, zone_path: None, query: None }),
                }
            })
            .collect(),
        Err(_) => vec![],
    }
}

fn save_history(history: &[HistoryEntry]) {
    let path = history_path();
    let content = history
        .iter()
        .map(|e| format!(
            "{}\t{}\t{}\t{}",
            e.added_at,
            e.zone_path.as_deref().unwrap_or(""),
            e.query.as_deref().unwrap_or(""),
            e.path.to_string_lossy()
        ))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(&path, content);
}

pub struct App {
    pub focus: Focus,
    pub zones: Vec<Zone>,
    pub zone_state: ListState,
    pub selected_file: Option<PathBuf>,
    pub action_state: ListState,
    pub history: Vec<HistoryEntry>,
    pub history_state: TableState,
    pub toast: Option<(String, Instant)>,
    pub cd_target: Option<PathBuf>,
    pub edit_target: Option<PathBuf>,
    pub fzf_running: bool,
    pub history_confirm: Option<HistoryConfirm>,
    pub ffmpeg_submenu: bool,
    pub ffmpeg_submenu_idx: usize,
    pub flash_action: Option<char>,
}

impl App {
    pub fn new() -> Self {
        let cfg = load_config();
        let mut zone_state = ListState::default();
        zone_state.select(Some(0));
        let mut action_state = ListState::default();
        action_state.select(Some(0));
        let history = load_history();
        let mut history_state = TableState::default();
        let selected_file = if !history.is_empty() {
            history_state.select(Some(0));
            Some(history[0].path.clone())
        } else {
            None
        };
        Self {
            focus: Focus::Zones,
            zones: Self::with_all_zone(cfg.zones),
            zone_state,
            selected_file,
            action_state,
            history,
            history_state,
            toast: None,
            cd_target: None,
            edit_target: None,
            fzf_running: false,
            history_confirm: None,
            ffmpeg_submenu: false,
            ffmpeg_submenu_idx: 0,
            flash_action: None,
        }
    }

    pub fn notify(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    /// The current toast message if it has not yet expired.
    pub fn active_toast(&self) -> Option<&str> {
        self.toast.as_ref().and_then(|(msg, at)| {
            (at.elapsed() < TOAST_DURATION).then_some(msg.as_str())
        })
    }

    pub fn zone_panel_width(&self) -> u16 {
        let max = self.zones.iter()
            .map(|z| z.name.len().max(12) + z.path.len())
            .max()
            .unwrap_or(10) as u16;
        (max + 3).clamp(16, 50)
    }

    pub fn actions_panel_width() -> u16 {
        let max = ACTIONS_FILE.iter()
            .map(|a| 4 + a.label.len())
            .max()
            .unwrap_or(20) as u16;
        max + 4
    }

    pub fn current_zone_path(&self) -> &str {
        let i = self.zone_state.selected().unwrap_or(0);
        self.zones.get(i).map(|z| z.path.as_str()).unwrap_or("/home/user")
    }

    pub fn with_all_zone(mut zones: Vec<Zone>) -> Vec<Zone> {
        zones.insert(0, Zone { name: "all".into(), path: "*".into() });
        zones
    }

    pub fn reload_zones(&mut self) {
        let cfg = load_config();
        let prev = self.zone_state.selected().unwrap_or(0);
        self.zones = Self::with_all_zone(cfg.zones);
        self.zone_state.select(Some(prev.min(self.zones.len().saturating_sub(1))));
    }

    pub fn current_actions(&self) -> &[Action] {
        match &self.selected_file {
            Some(p) if p.is_dir() => ACTIONS_DIR,
            Some(_) | None => ACTIONS_FILE,
        }
    }

    pub fn is_all_zone_selected(&self) -> bool {
        let i = self.zone_state.selected().unwrap_or(0);
        self.zones.get(i).map(|z| z.path == "*").unwrap_or(false)
    }

    fn fzf_cmd_for_zone_and_query(&self, zone_path: &str, query: Option<&str>) -> String {
        let (fd_paths, label) = if zone_path == "*" {
            let paths = self.zones.iter()
                .filter(|z| z.path != "*")
                .map(|z| format!("--search-path {}", shell_escape(&z.path)))
                .collect::<Vec<_>>()
                .join(" ");
            (paths, "All zones")
        } else {
            (format!("--search-path {}", shell_escape(zone_path)), "Pick")
        };
        let query_arg = query
            .filter(|q| !q.is_empty())
            .map(|q| format!(" --query {}", shell_escape(q)))
            .unwrap_or_default();
        format!(
            "{} --hidden --no-ignore {} \
             -E .wine -E .java -E .thunderbird -E .mozilla -E .git -E node_modules -E obj \
             | $HOME/.fzf/bin/fzf --border rounded --border-label ' {} ' --border-label-pos 2 --color 'label:yellow' \
                   --header '↑/↓ ctrl+k/j/p/n: Navigate    Enter: Select    ctrl+o/alt+Enter: Open    Esc/ctrl+c: Cancel' \
                   --expect ctrl-o,alt-enter \
                   --print-query{} \
                   --preview {} --preview-window=right:50%:border-left \
             > /tmp/easy-search_result",
            fd_binary(), fd_paths, label, query_arg, preview_cmd()
        )
    }

    pub fn fzf_cmd(&self) -> String {
        self.fzf_cmd_for_zone_and_query(self.current_zone_path(), None)
    }

    fn infer_zone_for_path(&self, path: &std::path::Path) -> Option<String> {
        self.zones.iter()
            .filter(|z| z.path != "*")
            .find(|z| path.starts_with(&z.path))
            .map(|z| z.path.clone())
    }

    pub fn retrigger_selected_history(&self) -> Option<(String, String)> {
        let entry = self.history_state.selected()
            .and_then(|i| self.history.get(i))?;
        let zone = entry.zone_path.clone()
            .or_else(|| self.infer_zone_for_path(&entry.path))
            .unwrap_or_else(|| self.current_zone_path().to_string());
        let cmd = self.fzf_cmd_for_zone_and_query(&zone, entry.query.as_deref());
        Some((cmd, zone))
    }

    pub fn apply_fzf_result(&mut self, zone_path: &str) {
        if let Ok(content) = std::fs::read_to_string("/tmp/easy-search_result") {
            let mut lines = content.lines();
            let query = lines.next().unwrap_or("").trim().to_string();
            let key = lines.next().unwrap_or("").trim().to_string();
            let path_str = lines.next().unwrap_or("").trim().to_string();
            if !path_str.is_empty() {
                let path = PathBuf::from(&path_str);
                self.history.retain(|e| e.path != path);
                self.history.insert(0, HistoryEntry {
                    path: path.clone(),
                    added_at: now_unix(),
                    zone_path: if zone_path.is_empty() { None } else { Some(zone_path.to_string()) },
                    query: if query.is_empty() { None } else { Some(query) },
                });
                self.history_state.select(Some(0));
                save_history(&self.history);
                self.selected_file = Some(path);
                self.action_state.select(Some(0));
                self.focus = Focus::Actions;
                self.toast = None;
                self.select_zone_by_path(zone_path);
                if key == "alt-enter" || key == "ctrl-o" {
                    let actions = if self.selected_file.as_ref().map_or(false, |p| p.is_dir()) {
                        ACTIONS_DIR
                    } else {
                        ACTIONS_FILE
                    };
                    if let Some(open_action) = actions.iter().find(|a| a.key == 'o') {
                        let _ = self.run_action(open_action);
                    }
                }
            }
        }
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.history_state.select(None);
        self.selected_file = None;
        save_history(&self.history);
    }

    pub fn delete_selected_history_entry(&mut self) {
        if let Some(i) = self.history_state.selected() {
            if i < self.history.len() {
                self.history.remove(i);
                save_history(&self.history);
                if self.history.is_empty() {
                    self.history_state.select(None);
                    self.selected_file = None;
                } else {
                    let new_i = i.min(self.history.len() - 1);
                    self.history_state.select(Some(new_i));
                    self.sync_selected_from_history();
                }
            }
        }
    }

    pub fn sync_selected_from_history(&mut self) {
        self.selected_file = self.history_state.selected()
            .and_then(|i| self.history.get(i))
            .map(|e| e.path.clone());
        self.action_state.select(Some(0));
    }

    pub fn select_zone_by_path(&mut self, zone_path: &str) {
        if let Some(i) = self.zones.iter().position(|z| z.path == zone_path) {
            self.zone_state.select(Some(i));
        }
    }

    pub fn select_history_item(&mut self) {
        if self.selected_file.is_some() {
            self.focus = Focus::Actions;
        }
    }

    pub fn run_action(&mut self, action: &Action) -> Result<()> {
        let path = match &self.selected_file {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        match action.key {
            'o' => {
                Command::new("xdg-open").arg(&path)
                    .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
                self.notify(format!("Opened {}", path.display()));
            }
            'e' => {
                self.edit_target = Some(path.clone());
                self.notify(format!("Editing {}", path.display()));
            }
            'd' => {
                let target = if path.is_dir() { path.clone() } else { path.parent().unwrap_or(&path).to_path_buf() };
                self.cd_target = Some(target);
            }
            'c' => {
                copy_to_clipboard(&path.to_string_lossy())?;
                self.notify("Path copied");
            }
            'n' => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                copy_to_clipboard(&name)?;
                self.notify("Name copied");
            }
            'r' => {
                self.ffmpeg_submenu = true;
                self.ffmpeg_submenu_idx = 0;
            }
            'p' => {
                let path_str = path.to_string_lossy();
                let path_str = path_str.trim_end_matches('/');
                Command::new("dolphin").args(["--select", path_str])
                    .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
                self.notify(format!("Opened {}", path.display()));
            }
            _ => {}
        }
        Ok(())
    }

    pub fn ffmpeg_subaction_preview(&self) -> Option<&'static str> {
        let key = FFMPEG_SUBACTIONS.get(self.ffmpeg_submenu_idx)?.key;
        build_ffmpeg_preview(key)
    }

    pub fn open_zone_folder(&mut self) -> Result<()> {
        let path = self.current_zone_path();
        if path == "*" {
            return Ok(());
        }
        let path = path.to_string();
        Command::new("xdg-open").arg(&path)
            .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
        self.notify(format!("Opened {}", path));
        Ok(())
    }

    pub fn cd_to_zone(&mut self) {
        let path = self.current_zone_path();
        if path != "*" {
            self.cd_target = Some(PathBuf::from(path));
        }
    }

    pub fn run_ffmpeg_subaction(&mut self, key: char) -> Result<()> {
        let path = match &self.selected_file {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        if let Some(cmd) = build_ffmpeg_command(&path, key) {
            copy_to_clipboard(&cmd)?;
            self.notify("ffmpeg command copied");
            self.ffmpeg_submenu = false;
        }
        Ok(())
    }
}
