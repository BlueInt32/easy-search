# Easy Search

> **Work in progress** — not ready for general use.

A keyboard-driven TUI file finder and launcher for Linux, built with [ratatui](https://github.com/ratatui-org/ratatui).

Pick a file with fzf, then act on it — open, edit, cd, copy.

```
┌Zones──────────────────┬History──────────────────────────┬/mnt/books/dune.epub──┐
│ all        *          │ dune.epub          /mnt/books    │ [o] open             │
│ home       ~          │ notes.md           ~/docs        │ [p] open parent      │
│ musique    /mnt/Music │ main.rs            ~/dev/foo/src │ [e] edit with kate   │
│ books      /mnt/books │                                  │ [d] cd               │
│                       │                                  │ [c] copy path        │
│                       │                                  │ [n] copy filename    │
└───────────────────────┴──────────────────────────────────┴──────────────────────┘
[j/k] Navigate  [Enter/f] Run search  [Tab/l] → history  [Z] Configure zones
```

## Features

- **Zones** — named bookmarks for your most-used directories; switch with `j/k`
- **All zone** — search across every configured zone at once
- **fzf picker** — launches in a tmux split (or inline as fallback) with file preview
- **History** — middle panel showing recently picked files; navigate and re-select instantly
- **Actions** — context-aware: different actions for files vs. directories
- **cd support** — writes the target to `/tmp/ratafzf_lastdir`; wire it to a shell function to actually `cd` there
- **Resizable panels** — drag either gutter to adjust panel widths

## Dependencies

### Required

- [`fd`](https://github.com/sharkdp/fd) — fast file finder used to index zones (`fdfind` on Debian/Ubuntu)
- [`fzf`](https://github.com/junegunn/fzf) — fuzzy finder powering the file picker

```sh
# Debian/Ubuntu
sudo apt install fd-find fzf

# Arch
sudo pacman -S fd fzf
```

### Optional

- **`tmux`** — fzf opens in a split pane; without it, easy-search falls back to suspending the TUI
- **`xclip`** — required for copy path / copy filename actions
- **`xdg-open`** — required for open / open parent folder actions
- **`kate`** — used by the edit action (swap in `src/actions.rs` if you prefer another editor)

## Install

```sh
cargo build --release
cp target/release/ratafzf ~/.local/bin/
```

### cd integration

Add this to your `.bashrc` / `.zshrc` to make `[d]` actually change your shell directory:

```sh
function es() {
    ratafzf
    if [ -f /tmp/ratafzf_lastdir ]; then
        cd "$(cat /tmp/ratafzf_lastdir)"
        rm /tmp/ratafzf_lastdir
    fi
}
```

## Configuration

Config lives at `~/.config/ratafzf.yaml` (created automatically on first run):

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
| `o/p/e/d/c/n` | Actions | run action by key |
| `Z` | everywhere | open config in `$EDITOR` |
| `q` | everywhere | quit |

## Built with

- [ratatui](https://github.com/ratatui-org/ratatui) — terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — terminal backend
- [serde](https://serde.rs) + [serde_yaml](https://github.com/dtolnay/serde-yaml) — config parsing
