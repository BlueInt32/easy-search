# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & run

```sh
cargo build                        # debug build
cargo build --release              # release build
cargo run                          # run directly
cargo clippy                       # lint
```

No tests exist yet. There is no test suite to run. Always run `cargo build --release` after making changes so the user can test them.

## Architecture

easy-search is a single-binary Rust TUI app (~6 source files) built with ratatui + crossterm.

### Module responsibilities

| File | Role |
|------|------|
| `main.rs` | Terminal setup/teardown, event loop (`run_app`), delegates to submodules |
| `app.rs` | `App` struct — all mutable state; fzf command building; history persistence |
| `input.rs` | Translates crossterm `Event` → `AppCommand` enum (no side effects) |
| `ui.rs` | Pure rendering: 3-panel layout + status bar + overlays |
| `actions.rs` | Static action lists (`ACTIONS_FILE`, `ACTIONS_DIR`, `FFMPEG_SUBACTIONS`) |
| `zones.rs` | `Zone`/`Config` structs, YAML config load/save |
| `ffmpeg.rs` | Reads last line of an ffmpeg log (minimal utility) |

### Data flow

**Event loop** (`main.rs::run_app`): `terminal.draw` → `input::handle_event` → `AppCommand` → mutate `App` → repeat.

**fzf picker**: if `$TMUX` is set, opens in a tmux split and waits via `tmux wait-for`; otherwise suspends the TUI. Result is read from `/tmp/easy-search_result`.

**cd action**: sets `app.cd_target` and returns early from `run_app`; `main` then writes the path to `/tmp/easy-search_lastdir` for the shell wrapper to pick up.

**edit action**: sets `app.edit_target`; `main.rs::edit_file` suspends the TUI and spawns `$EDITOR`.

**Config** is stored at `~/.config/easy-search.yaml`. History at `~/.config/easy-search_history`.

### Input modal layers

`input.rs::handle_key` checks modal state before routing to normal keybindings:
1. `app.history_confirm.is_some()` — confirmation dialog (y/Enter vs anything else)
2. `app.ffmpeg_submenu` — ffmpeg sub-menu navigation

### UI layout

Three horizontally-split panels with draggable gutters (widths stored in `app.zone_width` / `app.history_width`):
- **Zones** (left): list of configured zones + synthetic "all" zone prepended at index 0
- **History** (middle): recently picked files, persisted across runs
- **Actions** (right): context-sensitive — `ACTIONS_FILE` vs `ACTIONS_DIR` depending on `selected_file`

Two floating overlays rendered on top: ffmpeg submenu popup and history-confirm dialog.

## Language

All UI labels, status messages, and user-facing strings must be in English only. Never write labels or messages in French or any other language.

## Task backlog

Open tasks are tracked in the `easy-search-todo-obs/` Obsidian vault at the root of the repo.
