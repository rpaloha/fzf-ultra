mod cli;
mod daemon;
mod fuzzy;

use std::env;
use std::io::{self, BufRead, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn socket_path() -> PathBuf {
    let runtime = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::temp_dir());
    runtime.join("fzf-ultra.sock")
}

fn connect() -> io::Result<UnixStream> {
    let path = socket_path();
    for attempt in 0..60 {
        if let Ok(stream) = UnixStream::connect(&path) {
            return Ok(stream);
        }
        if attempt == 0 {
            let exe = env::current_exe()?;
            Command::new(exe)
                .arg("daemon")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "daemon nie uruchomił się w ciągu 1,5 s",
    ))
}

fn run_menu(placeholder: &str) -> io::Result<bool> {
    let items = io::read_to_string(io::stdin().lock())?;
    let mut stream = connect()?;
    writeln!(stream, "dmenu")?;
    writeln!(stream, "{}", placeholder.replace(['\r', '\n'], " "))?;
    stream.write_all(items.as_bytes())?;
    stream.shutdown(Shutdown::Write)?;
    let mut selection = String::new();
    let received = io::BufReader::new(stream).read_line(&mut selection)?;
    if received > 0 {
        print!("{selection}");
    }
    Ok(received > 0)
}

fn main() {
    let mut args = env::args().skip(1);
    let first = args.next().unwrap_or_else(|| "toggle".to_string());
    match first.as_str() {
        "--help" | "-h" => cli::usage(),
        "--version" | "-V" => println!("fzf-ultra {}", env!("CARGO_PKG_VERSION")),
        "daemon" => {
            if let Err(error) = daemon::run(socket_path()) {
                eprintln!("fzf-ultra daemon: {error}");
                std::process::exit(1);
            }
        }
        "--filter" | "filter" => {
            let Some(query) = args.next() else {
                eprintln!("fzf-ultra: --filter wymaga zapytania");
                std::process::exit(2);
            };
            let no_sort = args.any(|arg| arg == "--no-sort");
            cli::run_filter(&query, no_sort);
        }
        "dmenu" | "--dmenu" => {
            let placeholder = match (args.next().as_deref(), args.next()) {
                (None, None) => "Szukaj…".to_owned(),
                (Some("--placeholder"), Some(text)) if args.next().is_none() => text,
                _ => {
                    eprintln!("fzf-ultra: użyj --dmenu [--placeholder TEKST]");
                    std::process::exit(2);
                }
            };
            match run_menu(&placeholder) {
                Ok(true) => {}
                Ok(false) => std::process::exit(1),
                Err(error) => {
                    eprintln!("fzf-ultra: {error}");
                    std::process::exit(2);
                }
            }
        }
        "toggle" | "show" | "hide" => {
            if let Err(error) = connect().and_then(|mut stream| writeln!(stream, "{first}")) {
                eprintln!("fzf-ultra: {error}");
                std::process::exit(1);
            }
        }
        unknown => {
            eprintln!("fzf-ultra: nieznana komenda: {unknown}; użyj --help");
            std::process::exit(2);
        }
    }
}
