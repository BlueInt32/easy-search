#!/bin/bash

WINDOW_ID=$(wmctrl -l | grep " easy-search$" | awk '{print $1}' | head -1)

if [ -n "$WINDOW_ID" ]; then
    touch /tmp/easy-search_focus_zones  # signals the running app to reset focus to the Zones panel
    wmctrl -ia "$WINDOW_ID"
else
    kstart5 --maximize -- /home/simon/.cargo/bin/alacritty --title "easy-search" --class easy-search,easy-search --config-file ~/.config/easy-search-alacritty.toml \
        -e bash --login -c 'tmux new-session "/home/simon/dev/easy-search/target/release/easy-search --from-shortcut; dir=\$(cat /tmp/easy-search_lastdir 2>/dev/null); rm -f /tmp/easy-search_lastdir; [ -n \"\$dir\" ] && cd \"\$dir\"; exec bash --login -i"'
fi
