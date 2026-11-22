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

Bind a compositor hotkey to `~/.local/bin/fzf-ultra toggle` (expand the home path if the compositor requires it). The first invocation also starts the daemon if it is not running. Use `fzf-ultra show` or `fzf-ultra hide` for explicit control.

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

## License

MIT. See [LICENSE](LICENSE).
