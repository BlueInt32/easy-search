use anyhow::Result;
use ratatui::widgets::ListState;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::actions::{Action, ACTIONS_DIR, ACTIONS_FILE, copy_to_clipboard};
use crate::zones::{Zone, load_config};

#[derive(PartialEq)]
pub enum Focus {
    Zones,
    Actions,
}

pub struct App {
    pub focus: Focus,
    pub zones: Vec<Zone>,
    pub zone_state: ListState,
    pub selected_file: Option<PathBuf>,
    pub action_state: ListState,
    pub history: Vec<PathBuf>,
    pub status: Option<String>,
    pub ffmpeg_log: Option<String>,
    pub ffmpeg_child: Option<std::process::Child>,
    pub cd_target: Option<PathBuf>,
    pub zone_width: u16,
    pub dragging: bool,
    pub hover_gutter: bool,
}

impl App {
    pub fn new() -> Self {
        let cfg = load_config();
        let mut zone_state = ListState::default();
        zone_state.select(Some(0));
        let mut action_state = ListState::default();
        action_state.select(Some(0));
        Self {
            focus: Focus::Zones,
            zones: Self::with_all_zone(cfg.zones),
            zone_state,
            selected_file: None,
            action_state,
            history: vec![],
            status: None,
            ffmpeg_log: None,
            ffmpeg_child: None,
            cd_target: None,
            zone_width: 30,
            dragging: false,
            hover_gutter: false,
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
             | fzf --border rounded --border-label ' {} ' --border-label-pos 2 \
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
                self.history.push(path);
                self.action_state.select(Some(0));
                self.focus = Focus::Actions;
                self.status = None;
            }
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
                let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                let output = path.with_file_name(format!("{}-reenc.mp4", stem));
                let log_path = "/tmp/ratafzf_ffmpeg.log";
                let log_file = std::fs::File::create(log_path)?;
                let child = Command::new("ffmpeg")
                    .args([
                        "-i", &path.to_string_lossy(),
                        "-c:v", "libx264", "-crf", "18", "-preset", "slow",
                        "-c:a", "copy", &output.to_string_lossy(),
                    ])
                    .stdout(Stdio::null()).stderr(log_file)
                    .spawn()?;
                self.ffmpeg_child = Some(child);
                self.ffmpeg_log = Some(log_path.to_string());
                self.status = Some(format!("Encoding → {}", output.display()));
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
}
