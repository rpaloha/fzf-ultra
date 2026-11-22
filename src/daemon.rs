use crate::fuzzy;
use futures_channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use gio::prelude::*;
use gtk::gdk;
use gtk::prelude::*;
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

const MAX_RESULTS: usize = 9;
const STYLE: &str = include_str!("../assets/style.css");

enum Request {
    Toggle,
    Show,
    Hide,
    Menu(Menu),
}

struct Menu {
    placeholder: String,
    items: Vec<String>,
    reply: UnixStream,
}

struct AppEntry {
    info: gio::AppInfo,
    name: String,
    description: String,
    search_text: String,
    icon: Option<gio::Icon>,
}

fn app_entries() -> Vec<AppEntry> {
    let mut apps: Vec<AppEntry> = gio::AppInfo::all()
        .into_iter()
        .filter(|info| info.should_show())
        .filter_map(|info| {
            let name = info.display_name().to_string();
            if name.trim().is_empty() {
                return None;
            }
            let description = info
                .description()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let executable = info
                .executable()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let search_text = format!("{name} {description} {executable}");
            Some(AppEntry {
                icon: info.icon(),
                info,
                name,
                description,
                search_text,
            })
        })
        .collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    apps
}

fn parse_request(input: &str) -> Option<Request> {
    match input.trim() {
        "toggle" => Some(Request::Toggle),
        "show" => Some(Request::Show),
        "hide" => Some(Request::Hide),
        _ => None,
    }
}

fn listen(path: &Path, sender: UnboundedSender<Request>) -> io::Result<SocketCleanup> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "daemon już działa",
            ));
        }
        fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    thread::spawn(move || {
        for connection in listener.incoming() {
            let Ok(stream) = connection else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(50)));
            let mut reader = io::BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let request = if line.trim() == "dmenu" {
                let mut placeholder = String::new();
                if reader.read_line(&mut placeholder).unwrap_or(0) == 0 {
                    continue;
                }
                let Ok(items) = reader.by_ref().lines().collect::<io::Result<Vec<_>>>() else {
                    continue;
                };
                Some(Request::Menu(Menu {
                    placeholder: placeholder.trim_end().to_owned(),
                    items,
                    reply: reader.into_inner(),
                }))
            } else {
                parse_request(&line)
            };
            if let Some(request) = request {
                if sender.unbounded_send(request).is_err() {
                    break;
                }
            }
        }
    });
    Ok(SocketCleanup(path.to_path_buf()))
}

struct SocketCleanup(PathBuf);
impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

struct Ui {
    window: gtk::ApplicationWindow,
    search: gtk::Entry,
    list: gtk::ListBox,
    apps: Vec<AppEntry>,
    visible: Vec<usize>,
    menu: Option<Menu>,
    scroll: gtk::ScrolledWindow,
    _apps_monitor: gio::AppInfoMonitor,
}

impl Ui {
    fn new(app: &gtk::Application) -> Self {
        let window = gtk::ApplicationWindow::new(app);
        window.set_title(Some("fzf-ultra"));
        window.set_decorated(false);
        window.set_resizable(false);
        window.add_css_class("launcher-window");
        window.init_layer_shell();
        window.set_namespace(Some("fzf-ultra"));
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::Exclusive);
        window.set_exclusive_zone(0);
        window.set_default_size(460, -1);

        let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
        panel.add_css_class("launcher");

        let search = gtk::Entry::new();
        search.add_css_class("search");
        search.set_placeholder_text(Some("Szukaj aplikacji…"));
        search.set_hexpand(true);
        panel.append(&search);

        let list = gtk::ListBox::new();
        list.add_css_class("results");
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_activate_on_single_click(false);
        let scroll = gtk::ScrolledWindow::new();
        scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroll.set_min_content_height(130);
        scroll.set_max_content_height(280);
        scroll.set_propagate_natural_height(true);
        scroll.set_child(Some(&list));
        panel.append(&scroll);

        let footer = gtk::Label::new(Some("↑ ↓ wybierz    Enter zatwierdź    Esc zamknij"));
        footer.add_css_class("footer");
        footer.set_halign(gtk::Align::Start);
        panel.append(&footer);

        window.set_child(Some(&panel));

        let apps = app_entries();
        let visible = Vec::new();
        Self {
            window,
            search,
            list,
            apps,
            visible,
            menu: None,
            scroll,
            _apps_monitor: gio::AppInfoMonitor::get(),
        }
    }

    fn refresh(&mut self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        self.visible.clear();

        let query = self.search.text().to_string();
        let app_search;
        let search_values = if let Some(menu) = &self.menu {
            &menu.items
        } else {
            app_search = self
                .apps
                .iter()
                .map(|app| app.search_text.clone())
                .collect();
            &app_search
        };
        let found = fuzzy::matches(&query, search_values, false);
        let limit = if self.menu.is_some() {
            found.len()
        } else {
            MAX_RESULTS
        };
        for result in found.into_iter().take(limit) {
            let row = gtk::ListBoxRow::new();
            row.add_css_class("result-row");
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let labels = gtk::Box::new(gtk::Orientation::Vertical, 2);
            let mut description_text = "";
            let title_text = if let Some(menu) = &self.menu {
                &menu.items[result.index]
            } else {
                let app = &self.apps[result.index];
                let image = match &app.icon {
                    Some(icon) => gtk::Image::from_gicon(icon),
                    None => gtk::Image::from_icon_name("application-x-executable-symbolic"),
                };
                image.set_pixel_size(30);
                line.append(&image);
                description_text = &app.description;
                &app.name
            };
            let title = gtk::Label::new(Some(title_text));
            title.add_css_class("app-name");
            title.set_xalign(0.0);
            title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            labels.append(&title);
            if !description_text.is_empty() {
                let description = gtk::Label::new(Some(description_text));
                description.add_css_class("app-description");
                description.set_xalign(0.0);
                description.set_ellipsize(gtk::pango::EllipsizeMode::End);
                labels.append(&description);
            }
            line.append(&labels);
            row.set_child(Some(&line));
            self.list.append(&row);
            self.visible.push(result.index);
        }

        self.scroll.vadjustment().set_value(0.0);
        if self.visible.is_empty() {
            let row = gtk::ListBoxRow::new();
            row.set_selectable(false);
            let message = gtk::Label::new(Some("Brak wyników"));
            message.add_css_class("no-results");
            row.set_child(Some(&message));
            self.list.append(&row);
        } else if let Some(row) = self.list.row_at_index(0) {
            self.list.select_row(Some(&row));
        }
    }

    fn show(&mut self, menu: Option<Menu>) {
        self.menu = menu;
        self.search.set_placeholder_text(Some(
            self.menu
                .as_ref()
                .map_or("Szukaj aplikacji…", |menu| &menu.placeholder),
        ));
        self.search.set_text("");
        self.refresh();
        self.window.set_visible(true);
        self.window.present();
        self.search.grab_focus();
    }

    fn hide(&mut self) {
        self.menu = None;
        self.window.set_visible(false);
    }

    fn move_selection(&self, delta: i32) {
        if self.visible.is_empty() {
            return;
        }
        let current = self.list.selected_row().map(|row| row.index()).unwrap_or(0);
        let next = (current + delta).clamp(0, self.visible.len() as i32 - 1);
        if let Some(row) = self.list.row_at_index(next) {
            self.list.select_row(Some(&row));
            let adjustment = self.scroll.vadjustment();
            let top = row.allocation().y() as f64;
            adjustment.clamp_page(top, top + f64::from(row.height()));
        }
    }

    fn launch_selected(&mut self) {
        let row = self.list.selected_row().map(|row| row.index()).unwrap_or(0);
        let Some(app_index) = self.visible.get(row as usize).copied() else {
            return;
        };
        if let Some(menu) = &mut self.menu {
            let _ = writeln!(menu.reply, "{}", menu.items[app_index]);
            self.hide();
            return;
        }
        let app = &self.apps[app_index];
        match app.info.launch(&[], None::<&gio::AppLaunchContext>) {
            Ok(()) => self.hide(),
            Err(error) => eprintln!("fzf-ultra: nie udało się uruchomić {}: {error}", app.name),
        }
    }
}

fn load_css() -> String {
    let path = glib::user_config_dir().join("fzf-ultra/style.css");
    fs::read_to_string(path).unwrap_or_else(|_| STYLE.to_string())
}

fn build_ui(app: &gtk::Application, mut receiver: UnboundedReceiver<Request>) {
    let ui = Rc::new(RefCell::new(Ui::new(app)));
    ui.borrow_mut().refresh();
    let weak_ui = Rc::downgrade(&ui);

    {
        let weak = weak_ui.clone();
        ui.borrow().search.connect_changed(move |_| {
            if let Some(ui) = weak.upgrade() {
                if let Ok(mut ui) = ui.try_borrow_mut() {
                    ui.refresh();
                }
            }
        });
    }
    {
        let weak = weak_ui.clone();
        ui.borrow().list.connect_row_activated(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                ui.borrow_mut().launch_selected();
            }
        });
    }
    {
        let weak = weak_ui.clone();
        ui.borrow().search.connect_activate(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.borrow_mut().launch_selected();
            }
        });
    }
    {
        let weak = weak_ui.clone();
        ui.borrow().window.connect_close_request(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.borrow_mut().hide();
            }
            glib::Propagation::Stop
        });
    }
    {
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = weak_ui.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            let Some(ui) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            match key {
                gdk::Key::Escape => {
                    ui.borrow_mut().hide();
                    glib::Propagation::Stop
                }
                gdk::Key::Down => {
                    ui.borrow().move_selection(1);
                    glib::Propagation::Stop
                }
                gdk::Key::Up => {
                    ui.borrow().move_selection(-1);
                    glib::Propagation::Stop
                }
                gdk::Key::Return | gdk::Key::KP_Enter => {
                    ui.borrow_mut().launch_selected();
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        ui.borrow().window.add_controller(key_controller);
    }
    let apps_monitor = ui.borrow()._apps_monitor.clone();
    {
        let weak = weak_ui.clone();
        apps_monitor.connect_changed(move |_| {
            if let Some(ui) = weak.upgrade() {
                let mut ui = ui.borrow_mut();
                ui.apps = app_entries();
                ui.refresh();
            }
        });
    }
    let _apps_monitor = apps_monitor;

    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_data(&load_css());
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // Keep the UI alive while hidden and wake GTK only when a request arrives.
    glib::MainContext::default().spawn_local(async move {
        while let Some(request) = receiver.next().await {
            match request {
                Request::Show => ui.borrow_mut().show(None),
                Request::Menu(menu) => ui.borrow_mut().show(Some(menu)),
                Request::Hide => ui.borrow_mut().hide(),
                Request::Toggle => {
                    if ui.borrow().window.is_visible() {
                        ui.borrow_mut().hide();
                    } else {
                        ui.borrow_mut().show(None);
                    }
                }
            }
        }
    });
}

pub fn run(socket: PathBuf) -> io::Result<()> {
    let (sender, receiver) = mpsc::unbounded();
    let cleanup = listen(&socket, sender)?;
    let app = gtk::Application::new(
        Some("dev.fzf_ultra.Launcher"),
        gio::ApplicationFlags::NON_UNIQUE,
    );
    let receiver = Rc::new(RefCell::new(Some(receiver)));
    app.connect_activate(move |app| {
        if let Some(receiver) = receiver.borrow_mut().take() {
            build_ui(app, receiver);
        }
    });
    // `daemon` is consumed by our CLI, so do not let GTK interpret it as a
    // file argument when it initializes the application.
    app.run_with_args(&[""]);
    drop(cleanup);
    Ok(())
}
