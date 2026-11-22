use crate::fuzzy;
use std::io::{self, BufRead, IsTerminal};

pub fn run_filter(query: &str, no_sort: bool) {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        eprintln!("fzf-ultra: podaj listę kandydatów przez stdin; użyj --help.");
        std::process::exit(2);
    }
    let items: Vec<String> = stdin
        .lock()
        .lines()
        .filter_map(Result::ok)
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect();
    for found in fuzzy::matches(query, &items, no_sort) {
        println!("{}", items[found.index]);
    }
}

pub fn usage() {
    println!(
        "fzf-ultra — szybki launcher Wayland z wyszukiwaniem fuzzy\n\nUżycie:\n  fzf-ultra [toggle]\n  fzf-ultra show | hide | daemon\n  lista | fzf-ultra --dmenu [--placeholder TEKST]\n  lista | fzf-ultra --filter ZAPYTANIE [--no-sort]\n\nSkróty w oknie:\n  ↑ / ↓       wybierz aplikację\n  Enter       uruchom\n  Esc         zamknij okno\n\nDaemon można uruchomić przez systemd --user."
    );
}
