use anyhow::Result;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct Action {
    pub key: char,
    pub label: &'static str,
}

pub const ACTIONS_FILE: &[Action] = &[
    Action { key: 'o', label: "open" },
    Action { key: 'p', label: "open parent folder" },
    Action { key: 'e', label: "edit with kate" },
    Action { key: 'd', label: "cd" },
    Action { key: 'c', label: "copy path" },
    Action { key: 'n', label: "copy filename" },
    Action { key: 'r', label: "ffmpeg" },
];

pub const FFMPEG_SUBACTIONS: &[Action] = &[
    Action { key: 'e', label: "encode to mp4 (good quality)" },
    Action { key: 'a', label: "extract audio" },
];

pub const ACTIONS_DIR: &[Action] = &[
    Action { key: 'o', label: "open" },
    Action { key: 'd', label: "cd" },
    Action { key: 'c', label: "copy path" },
    Action { key: 'n', label: "copy filename" },
];

pub fn copy_to_clipboard(s: &str) -> Result<()> {
    use std::io::Write;
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()?;
    child.stdin.as_mut().unwrap().write_all(s.as_bytes())?;
    child.wait()?;
    Ok(())
}
