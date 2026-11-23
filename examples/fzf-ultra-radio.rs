use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const STOP: &str = "Stop radio";

struct Station<'a> {
    name: &'a str,
    url: &'a str,
    profile: &'a str,
}

fn menu(placeholder: &str, choices: &str) -> io::Result<Option<String>> {
    let mut child = Command::new("fzf-ultra")
        .args(["--dmenu", "--placeholder", placeholder])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(choices.as_bytes())?;
    let output = child.wait_with_output()?;
    Ok(output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .trim_end_matches(['\n', '\r'])
            .to_owned()
    }))
}

fn connect(socket: &Path) -> io::Result<Option<UnixStream>> {
    match UnixStream::connect(socket) {
        Ok(stream) => Ok(Some(stream)),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn wait_until(socket: &Path, running: bool) -> io::Result<()> {
    for _ in 0..100 {
        if connect(socket)?.is_some() == running {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "Timed out waiting for the mpv radio socket",
    ))
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

fn run() -> io::Result<()> {
    let config = env::var_os("XDG_CONFIG_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| io::Error::other("No configuration directory"))?;
    let stations_file = config.join("fzf-ultra/stations.csv");
    let stop_requested = env::args_os().nth(1).is_some_and(|arg| arg == "--stop");
    let data = if stop_requested {
        String::new()
    } else {
        fs::read_to_string(&stations_file).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("{}: {error}", stations_file.display()),
            )
        })?
    };
    let stations: Vec<_> = data
        .lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once(',')?;
            let (url, profile) = rest.split_once(',').unwrap_or((rest, ""));
            if name.is_empty() || url.is_empty() {
                return None;
            }
            Some(Station { name, url, profile })
        })
        .collect();

    let selected = if stop_requested {
        None
    } else {
        if stations.is_empty() {
            menu("Radio", "No stations configured\n")?;
            return Ok(());
        }
        let mut choices = format!("{STOP}\n");
        for station in &stations {
            choices.push_str(station.name);
            choices.push('\n');
        }
        match menu("Radio", &choices)?.as_deref() {
            Some(STOP) => None,
            Some(name) => match stations.iter().find(|station| station.name == name) {
                Some(station) => Some(station),
                None => return Ok(()),
            },
            None => return Ok(()),
        }
    };

    let runtime = PathBuf::from(
        env::var_os("XDG_RUNTIME_DIR")
            .ok_or_else(|| io::Error::other("XDG_RUNTIME_DIR is unset"))?,
    );
    let socket = runtime.join("fzf-ultra-radio.sock");
    let current = runtime.join("fzf-ultra-radio.station");
    let lock = File::options()
        .write(true)
        .create(true)
        .truncate(false)
        .open(runtime.join("fzf-ultra-radio.lock"))?;
    lock.lock()?;

    let same_station = if let Some(mut stream) = connect(&socket)? {
        let same = selected.is_some_and(|station| {
            fs::read_to_string(&current).is_ok_and(|url| url == station.url)
        });
        stream.set_write_timeout(Some(Duration::from_secs(1)))?;
        stream.write_all(b"{\"command\":[\"quit\"]}\n")?;
        drop(stream);
        wait_until(&socket, false)?;
        same
    } else {
        false
    };
    remove_if_present(&socket)?;
    remove_if_present(&current)?;

    if let Some(station) = selected.filter(|_| !same_station) {
        fs::write(&current, station.url)?;
        let mut command = Command::new("mpv");
        command.arg("--no-video");
        if !station.profile.is_empty() {
            command.arg(format!("--profile={}", station.profile));
        }
        let mut player = command
            .arg(format!("--force-media-title={}", station.name))
            .arg(format!("--input-ipc-server={}", socket.display()))
            .args(["--", station.url])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        if let Err(error) = wait_until(&socket, true) {
            let _ = player.kill();
            let _ = player.wait();
            remove_if_present(&socket)?;
            remove_if_present(&current)?;
            return Err(error);
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    run()
}
