use anyhow::Result;
use ratatui::widgets::ListState;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::actions::{Action, ACTIONS_DIR, ACTIONS_FILE, FFMPEG_SUBACTIONS, copy_to_clipboard};
use crate::zones::{Zone, load_config};

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
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

fn history_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("ratafzf_history")
}

fn load_history() -> Vec<PathBuf> {
    let path = history_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => content
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect(),
        Err(_) => vec![],
    }
}

fn save_history(history: &[PathBuf]) {
    let path = history_path();
    let content = history
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
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
    pub history: Vec<PathBuf>,
    pub history_state: ListState,
    pub status: Option<String>,
    pub cd_target: Option<PathBuf>,
    pub zone_width: u16,
    pub history_width: u16,
    pub dragging: bool,
    pub dragging_right: bool,
    pub hover_gutter: bool,
    pub hover_right_gutter: bool,
    pub fzf_running: bool,
    pub history_confirm: Option<HistoryConfirm>,
    pub ffmpeg_submenu: bool,
    pub ffmpeg_submenu_idx: usize,
}

impl App {
    pub fn new() -> Self {
        let cfg = load_config();
        let mut zone_state = ListState::default();
        zone_state.select(Some(0));
        let mut action_state = ListState::default();
        action_state.select(Some(0));
        let history = load_history();
        let mut history_state = ListState::default();
        let selected_file = if !history.is_empty() {
            history_state.select(Some(0));
            Some(history[0].clone())
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
            status: None,
            cd_target: None,
            zone_width: 30,
            history_width: 40,
            dragging: false,
            dragging_right: false,
            hover_gutter: false,
            hover_right_gutter: false,
            fzf_running: false,
            history_confirm: None,
            ffmpeg_submenu: false,
            ffmpeg_submenu_idx: 0,
        }
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

    pub fn fzf_cmd(&self) -> String {
        let zone_path = self.current_zone_path();
        let (fd_paths, label) = if zone_path == "*" {
            let paths = self.zones.iter()
                .filter(|z| z.path != "*")
                .map(|z| format!("--search-path '{}'", z.path))
                .collect::<Vec<_>>()
                .join(" ");
            (paths, "All zones")
        } else {
            (format!("--search-path '{}'", zone_path), "Pick")
        };
        format!(
            "fdfind --hidden --no-ignore {} \
             -E .wine -E .java -E .thunderbird -E .mozilla -E .git -E node_modules -E obj \
             | fzf --border rounded --border-label ' {} ' --border-label-pos 2 --color 'label:yellow' \
                   --header '↑/↓ ctrl+k/j/p/n: Navigate    Enter: Select    Esc/ctrl+c: Cancel' \
                   --preview '~/.config/fzf/preview.sh {{}}' --preview-window=right:50%:border-left \
             > /tmp/ratafzf_result",
            fd_paths, label
        )
    }

    pub fn apply_fzf_result(&mut self) {
        if let Ok(content) = std::fs::read_to_string("/tmp/ratafzf_result") {
            let path_str = content.trim().to_string();
            if !path_str.is_empty() {
                let path = PathBuf::from(&path_str);
                self.selected_file = Some(path.clone());
                self.history.retain(|p| p != &path);
                self.history.insert(0, path);
                self.history_state.select(Some(0));
                save_history(&self.history);
                self.action_state.select(Some(0));
                self.focus = Focus::Actions;
                self.status = None;
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
            .cloned();
        self.action_state.select(Some(0));
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
                self.status = Some(format!("Opened {}", path.display()));
            }
            'e' => {
                Command::new("kate").arg(&path)
                    .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
                self.status = Some(format!("Editing {}", path.display()));
            }
            'd' => {
                let target = if path.is_dir() { path.clone() } else { path.parent().unwrap_or(&path).to_path_buf() };
                self.cd_target = Some(target);
            }
            'c' => {
                copy_to_clipboard(&path.to_string_lossy())?;
                self.status = Some("Chemin copié".to_string());
            }
            'n' => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                copy_to_clipboard(&name)?;
                self.status = Some("Nom copié".to_string());
            }
            'r' => {
                self.ffmpeg_submenu = true;
                self.ffmpeg_submenu_idx = 0;
            }
            'p' => {
                let parent = path.parent().unwrap_or(&path).to_path_buf();
                Command::new("xdg-open").arg(&parent)
                    .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
                self.status = Some(format!("Opened {}", parent.display()));
            }
            _ => {}
        }
        Ok(())
    }

    pub fn ffmpeg_subaction_preview(&self) -> Option<&'static str> {
        let key = FFMPEG_SUBACTIONS.get(self.ffmpeg_submenu_idx)?.key;
        build_ffmpeg_preview(key)
    }

    pub fn run_ffmpeg_subaction(&mut self, key: char) -> Result<()> {
        let path = match &self.selected_file {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        if let Some(cmd) = build_ffmpeg_command(&path, key) {
            copy_to_clipboard(&cmd)?;
            self.status = Some("Commande ffmpeg copiée".to_string());
            self.ffmpeg_submenu = false;
        }
        Ok(())
    }
}
