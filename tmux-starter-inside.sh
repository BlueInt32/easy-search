tmux new -d -s easy-search -n 'Easy Search' -c ~/dev/easy-search/
tmux send -t easy-search '' Enter

tmux neww -t easy-search -n 'Git' -c ~/dev/easy-search/
tmux send -t easy-search:'Git' 'g' Enter

tmux neww -t easy-search -n ' Claude' -c ~/dev/easy-search/
tmux send -t easy-search:' Claude' 'claude' Enter

tmux neww -t easy-search -n '  Run' -c ~/dev/easy-search/
tmux send -t easy-search:'  Run' 'cargo run' Enter

tmux set-window-option -g window-status-current-format " #W "
tmux set-window-option -g window-status-format " #W#(printf '%%s\n' '#F' | sed 's/-//') "

tmux select-window -t easy-search:'Easy Search'
tmux switch-client -t easy-search
