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

```sh
printf 'Stacja A\nStacja B\n' | fzf-ultra --dmenu --placeholder 'Radio'
```

The resident daemon displays the list and returns the selected line on stdout.
Escape (or `fzf-ultra hide`) cancels with exit status 1 and empty stdout.
The menu uses the same fuzzy search and keyboard controls as the app picker.

Use this mode from scripts that present a list of radio stations, videos, or
other actions. Pipe a newline-delimited list to the menu and use the selected
line as the action's input.

## License

MIT. See [LICENSE](LICENSE).
