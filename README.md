# fzf-ultra

A deliberately small Linux launcher: GTK4 + Wayland layer-shell UI, fzf-style fuzzy ranking, and one resident daemon that keeps the desktop app list warm. It has no plugin framework, file index, clipboard database, web search, or background file scanner.

The interface borrows the compact keyboard-first flow of Walker/Rofi and uses a Graphite-inspired dark theme. The daemon owns the GTK window and desktop-entry cache. Repeated keybind calls only send a short command over a local Unix socket, so the launcher does not cold-start GTK each time.

Tested on MangoWC. The compositor must support the Wayland layer-shell protocol.

## Build

System packages: `gtk4`, `gtk4-layer-shell`, `rust`, `cargo`.

```sh
git clone https://github.com/rpaloha/fzf-ultra.git
cd fzf-ultra
cargo build --release --locked
```

## Install for the current user

```sh
install -Dm755 target/release/fzf-ultra ~/.local/bin/fzf-ultra
install -Dm644 systemd/fzf-ultra.service ~/.config/systemd/user/fzf-ultra.service
systemctl --user daemon-reload
systemctl --user enable --now fzf-ultra.service
```

Run `~/.local/bin/fzf-ultra toggle` to try the launcher. The first invocation also starts the daemon if needed. Use `show` or `hide` for explicit control.

Theme override: copy `assets/style.css` to `~/.config/fzf-ultra/style.css` and edit it. The service can be disabled with `systemctl --user disable --now fzf-ultra.service`.

## CLI filter mode

The core fuzzy matcher can also filter arbitrary stdin lists:

```sh
printf 'Firefox\nHelium\nmpv\n' | fzf-ultra --filter fi
```

## Interactive stdin menu

Pass one item per line to `fzf-ultra --dmenu`. This example opens a picker
with two sample station names and displays “Radio” in the search field:

```sh
printf '%s\n' 'Station A' 'Station B' | fzf-ultra --dmenu --placeholder 'Radio'
```

Type to filter, use the arrow keys to choose an item, and press Enter.
Choosing **Station B** prints `Station B` to the terminal (stdout).
Pressing Escape cancels with exit status 1 and no output.

The station names are sample data. To play a station, a calling script uses
the selected line to look up a stream URL and passes it to a player such as mpv.

## Video picker with mpv

The example script recursively lists local videos, opens the selected file
in mpv, and exits without playing anything if you cancel. It requires Bash,
`find`, `sort`, and `mpv`.

```sh
install -Dm755 examples/fzf-ultra-videos ~/.local/bin/fzf-ultra-videos
~/.local/bin/fzf-ultra-videos
```

The default folder is `~/Videos`. To use another folder, pass it as the first
argument, for example `~/.local/bin/fzf-ultra-videos "$HOME/Movies"`.

## Desktop theme picker

If you already have a script that applies a desktop theme by name, use
`fzf-ultra` to select the theme. Save this example as
`~/.local/bin/pick-theme` and adjust the names and script path to match
your setup.

```sh
#!/bin/sh
theme=$(printf '%s\n' arch-stone mango-grove cosmos |
    "$HOME/.local/bin/fzf-ultra" --dmenu --placeholder 'Desktop theme') || exit 0
exec "$HOME/.config/theme-switcher/switch-theme.sh" "$theme"
```

Make it executable with `chmod +x ~/.local/bin/pick-theme`. To open it with
Super+T on MangoWC, add the following binding with your own home directory.

```ini
bind=SUPER,t,spawn,/home/YOUR_USER/.local/bin/pick-theme
```

### Matching launcher colors

The [example palettes](examples/themes) include Arch Stone, Mango Grove, and
Cosmos. Each has a slightly translucent panel. From the repository root,
install a palette and reload the launcher by restarting its user service.

```sh
config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/fzf-ultra"
install -Dm644 examples/themes/cosmos.css "$config_dir/style.css"
systemctl --user restart fzf-ultra.service
```

If you start the daemon manually, restart that process instead. To apply the
matching launcher colors automatically, run the same copy and restart from
your theme script after applying the desktop theme. Use the selected theme
name as the CSS filename.

## Keyboard shortcuts

The launcher has no global shortcut of its own. In your compositor's keybinding
settings, choose an available shortcut and bind it to one of these commands.
Use a full path if the compositor does not expand `~` or include `~/.local/bin`
in its search path.

| Action | Command |
| --- | --- |
| Toggle the app launcher | `~/.local/bin/fzf-ultra toggle` |
| Open the video picker | `~/.local/bin/fzf-ultra-videos` |

For example, in MangoWC, replace `/home/YOUR_USER` with your actual home
directory and reload the compositor configuration:

```ini
bind=SUPER,space,spawn,/home/YOUR_USER/.local/bin/fzf-ultra toggle
bind=ALT,v,spawn,/home/YOUR_USER/.local/bin/fzf-ultra-videos
```

These commands also work from a terminal in the Wayland session.

## License

MIT. See [LICENSE](LICENSE).
