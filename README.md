# Easy Search

> **Work in progress** — not ready for general use.

A keyboard-driven TUI file finder and launcher for Linux, built with [ratatui](https://github.com/ratatui-org/ratatui).

Pick a file with fzf, then act on it — open, edit, cd, copy.

```
┌Zones──────────────────┬History──────────────────────────┬/mnt/books/dune.epub──┐
│ all        *          │ dune.epub          /mnt/books    │ [o] open             │
│ home       ~          │ notes.md           ~/docs        │ [p] show in folder   │
│ musique    /mnt/Music │ main.rs            ~/dev/foo/src │ [e] edit with $EDITOR│
│ books      /mnt/books │                                  │ [d] cd               │
│                       │                                  │ [c] copy path        │
│                       │                                  │ [n] copy filename    │
│                       │                                  │ [r] ffmpeg           │
└───────────────────────┴──────────────────────────────────┴──────────────────────┘
[j/k] Navigate  [Enter/f] Run search  [Tab/l] → history  [Z] Configure zones
```

## Features

- **Zones** — named bookmarks for your most-used directories; switch with `j/k`
- **All zone** — search across every configured zone at once
- **fzf picker** — launches in a tmux split (or inline as fallback) with file preview
- **History** — middle panel showing recently picked files; navigate and re-select instantly
- **Actions** — context-aware: different actions for files vs. directories; file actions include an ffmpeg submenu (`[r]`) that copies a ready-to-run command to the clipboard
- **cd support** — writes the target to `/tmp/easy-search_lastdir`; wire it to a shell function to actually `cd` there

## Dependencies

### Required

- [`fd`](https://github.com/sharkdp/fd) — fast file finder used to index zones (`fdfind` on Debian/Ubuntu)
- [`fzf`](https://github.com/junegunn/fzf) — fuzzy finder powering the file picker; must be installed via the [fzf git method](https://github.com/junegunn/fzf#using-git) so it lands at `~/.fzf/` (the binary is referenced as `$HOME/.fzf/bin/fzf`)

```sh
# Debian/Ubuntu (fd)
sudo apt install fd-find

# Arch (fd + fzf via pacman)
sudo pacman -S fd fzf

# fzf via git (required on all distros if not using Arch's package)
git clone --depth 1 https://github.com/junegunn/fzf.git ~/.fzf && ~/.fzf/install
```

### Optional

- **`tmux`** — fzf opens in a `display-popup`; without it, easy-search falls back to suspending the TUI
- **`xclip`** — required for copy path / copy filename / ffmpeg actions
- **`xdg-open`** — required for the open action
- **`dolphin`** — required for the "show in folder" action (`[p]`)
- **`$EDITOR`** — used by the edit action; falls back to `nvim` if unset

## Install

```sh
cargo build --release
cp target/release/easy-search ~/.local/bin/
```

### Desktop integration (KDE + Alacritty)

Gives easy-search its own taskbar icon, window title, and window decoration icon instead of appearing as a generic Alacritty instance.

**1. Alacritty config** — create `~/.config/easy-search-alacritty.toml`:

```toml
[general]
import = ["~/.config/alacritty.toml"]

[window]
title = "easy-search"
dynamic_title = false
```

**2. Launch script** — `launch-easy-search.sh` is already in this repo. Symlink it to `~/scripts/`:

```sh
ln -s ~/dev/easy-search/launch-easy-search.sh ~/scripts/launch-easy-search.sh
```

**3. Desktop entry** — create `~/.local/share/applications/easy-search.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=Easy Search
Comment=File search TUI
Exec=/home/simon/scripts/launch-easy-search.sh
Icon=/home/simon/.local/share/icons/hicolor/scalable/apps/easy-search.svg
StartupWMClass=easy-search
NoDisplay=false
Terminal=false
Categories=Utility;FileTools;
```

**4. Icon** — symlink `easy-search.svg` from this repo to the system icon path:

```sh
mkdir -p ~/.local/share/icons/hicolor/scalable/apps
ln -s ~/dev/easy-search/easy-search.svg ~/.local/share/icons/hicolor/scalable/apps/easy-search.svg
```

**5. KWin rule** — append to `~/.config/kwinrulesrc` (increment `count` and `rules` in `[General]`):

```ini
[N]
Description=Force easy-search icon
desktopfile=easy-search
desktopfilerule=3
wmclass=easy-search
wmclassmatch=1
```

**6. Apply** — rebuild caches and reload KWin:

```sh
mkdir -p ~/.local/share/icons/hicolor/scalable/apps ~/.local/share/applications
gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor/
update-desktop-database ~/.local/share/applications/
kbuildsycoca5 --noincremental
qdbus org.kde.KWin /KWin reconfigure
```

### cd integration

Add this to your `.bashrc` / `.zshrc` to make `[d]` actually change your shell directory:

```sh
function es() {
    easy-search
    if [ -f /tmp/easy-search_lastdir ]; then
        cd "$(cat /tmp/easy-search_lastdir)"
        rm /tmp/easy-search_lastdir
    fi
}
```

## Configuration

Config lives at `~/.config/easy-search.yaml` (created automatically on first run):

```yaml
zones:
  - name: home
    path: /home/yourname
  - name: projects
    path: /home/yourname/dev
  - name: music
    path: /mnt/Music
```

Press `[Z]` inside easy-search to open the config in your `$EDITOR`.

## Key bindings

| Key | Context | Action |
|-----|---------|--------|
| `j` / `k` | everywhere | navigate up/down |
| `Tab` / `l` | everywhere | focus next panel (Zones → History → Actions) |
| `h` | History / Actions | focus previous panel |
| `f` | everywhere | open fzf picker on the selected zone |
| `Enter` | Zones | open fzf picker |
| `Enter` | History | select entry and move to Actions |
| `x` | History | delete the selected history entry |
| `X` | History | clear all history |
| `Enter` | Actions | run the selected action |
| `o` | Actions | open with `xdg-open` |
| `p` | Actions | show in folder (opens Dolphin with file selected) |
| `e` | Actions | edit with `$EDITOR` |
| `d` | Actions | cd to file's directory |
| `c` / `n` | Actions | copy full path / copy filename |
| `r` | Actions (file) | open ffmpeg submenu |
| `Z` | everywhere | open config in `$EDITOR` |
| `q` | everywhere | quit |

### ffmpeg submenu (`[r]`)

| Key | Action |
|-----|--------|
| `j` / `k` | navigate subactions |
| `Enter` | run selected subaction |
| `e` | encode to mp4 (libx264, CRF 18) |
| `a` | extract audio to mp3 |
| `Esc` | close submenu |

Subactions copy a ready-to-run ffmpeg command to the clipboard rather than executing it.

## Built with

- [ratatui](https://github.com/ratatui-org/ratatui) — terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — terminal backend
- [serde](https://serde.rs) + [serde_yaml](https://github.com/dtolnay/serde-yaml) — config parsing
