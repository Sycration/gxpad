//! sgcli: an interactive command line for sgpad's geometry. A line is a
//! statement of the .sgp format, which sggeo reads into the document, or
//! one of the few commands of the session (`?`, `ls`, `undo`, `save`, ...).

mod cmd;
mod help;
mod plot;

use std::io::{BufRead, IsTerminal};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut s = cmd::Session::new();
    let mut commands = Vec::new();
    let mut failed = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("usage: sgcli [FILE.sgp] [-c COMMAND]...\n\nWith no commands, sgcli reads commands from the terminal (or from standard input).\n`help` in sgcli lists them.");
                return;
            }
            "-c" => {
                let Some(c) = args.get(i + 1) else {
                    eprintln!("sgcli: -c needs a command");
                    std::process::exit(2);
                };
                commands.push(c.clone());
                i += 1;
            }
            file => failed |= !run(&mut s, &format!("open {file}")),
        }
        i += 1;
    }
    if !commands.is_empty() {
        for c in &commands {
            failed |= !run(&mut s, c);
        }
        std::process::exit(failed as i32);
    }
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        prompt(&mut s);
        return;
    }
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if !line.trim().is_empty() {
            println!("sg> {line}");
        }
        if matches!(line.trim(), "quit" | "exit") {
            break;
        }
        failed |= !run(&mut s, &line);
    }
    std::process::exit(failed as i32);
}

/// Reads commands at a prompt with line editing and a history kept between
/// sessions (in ~/.sgcli_history). Ctrl-C drops the line being typed, Ctrl-D quits.
fn prompt(s: &mut cmd::Session) {
    use rustyline::error::ReadlineError;
    println!("sgcli {}: geometry in the statements of .sgp files. `help` lists the commands.", env!("CARGO_PKG_VERSION"));
    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(rl) => rl,
        Err(e) => {
            eprintln!("sgcli: no line editing ({e})");
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                if matches!(line.trim(), "quit" | "exit") {
                    break;
                }
                run(s, &line);
            }
            return;
        }
    };
    let history = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(|h| std::path::PathBuf::from(h).join(".sgcli_history"));
    if let Some(h) = &history {
        let _ = rl.load_history(h);
    }
    loop {
        match rl.readline("sg> ") {
            Ok(line) => {
                if !line.trim().is_empty() {
                    let _ = rl.add_history_entry(line.as_str());
                }
                if matches!(line.trim(), "quit" | "exit") {
                    break;
                }
                run(s, &line);
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("sgcli: {e}");
                break;
            }
        }
    }
    if let Some(h) = &history {
        let _ = rl.save_history(h);
    }
}

/// Runs one line and prints what it did; false when it failed.
fn run(s: &mut cmd::Session, line: &str) -> bool {
    match s.run(line) {
        Ok(out) => {
            for l in out {
                if l.is_empty() {
                    println!();
                } else {
                    println!("  {l}");
                }
            }
            true
        }
        Err(e) => {
            println!("  ! {e}");
            false
        }
    }
}
