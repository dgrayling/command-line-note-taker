mod store;

use chrono::{Local, TimeZone};
use clap::Parser;
use std::io::{IsTerminal, Read};
use std::path::PathBuf;
use std::process::ExitCode;
use store::{Note, Store};

/// Jot down notes from the terminal and search them later.
///
/// Examples:
///   note call the dentist on friday
///   note -s dentist
///   note -l 50
///   echo "longer text" | note -
#[derive(Parser)]
#[command(name = "note", version, verbatim_doc_comment)]
struct Cli {
    /// Search notes: all words must match; end a word with * for prefix match (e.g. dent*)
    #[arg(short, long, conflicts_with_all = ["list", "delete"])]
    search: bool,

    /// List the last N notes (default 20)
    #[arg(short, long, value_name = "N", num_args = 0..=1, default_missing_value = "20",
          conflicts_with = "delete")]
    list: Option<usize>,

    /// Delete the note with this id
    #[arg(short, long, value_name = "ID")]
    delete: Option<i64>,

    /// The note text (or search words with -s). Use "-" to read the note from stdin.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    words: Vec<String>,
}

fn db_path() -> Result<PathBuf, String> {
    let path = match std::env::var_os("NOTE_DB") {
        Some(p) => PathBuf::from(p),
        None => dirs::home_dir()
            .ok_or("could not find home directory; set NOTE_DB")?
            .join(".local/share/note/notes.db"),
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    Ok(path)
}

fn print_notes(notes: &[Note]) {
    for n in notes {
        let when = Local
            .timestamp_opt(n.created_at, 0)
            .single()
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        // Indent continuation lines of multi-line notes under the body column.
        let body = n.body.replace('\n', "\n                         ");
        println!("#{:<4} {}  {}", n.id, when, body);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let store = Store::open(&db_path()?).map_err(|e| e.to_string())?;
    let err = |e: rusqlite::Error| e.to_string();

    if let Some(id) = cli.delete {
        return match store.delete(id).map_err(err)? {
            Some(body) => {
                println!("Deleted #{id}: {body}");
                Ok(())
            }
            None => Err(format!("no note with id #{id}")),
        };
    }

    if let Some(n) = cli.list {
        print_notes(&store.recent(n).map_err(err)?);
        return Ok(());
    }

    if cli.search {
        if cli.words.is_empty() {
            return Err("give some words to search for, e.g. note -s dentist".into());
        }
        let hl = if std::io::stdout().is_terminal() { ("\x1b[1;33m", "\x1b[0m") } else { ("", "") };
        let results = store.search(&cli.words, hl).map_err(err)?;
        if results.is_empty() {
            println!("No notes match.");
        } else {
            print_notes(&results);
        }
        return Ok(());
    }

    if cli.words.is_empty() {
        print_notes(&store.recent(10).map_err(err)?);
        return Ok(());
    }

    let body = if cli.words == ["-"] {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).map_err(|e| e.to_string())?;
        s.trim().to_string()
    } else {
        cli.words.join(" ")
    };
    if body.is_empty() {
        return Err("note is empty".into());
    }
    let id = store.add(&body, Local::now().timestamp()).map_err(err)?;
    println!("Saved #{id}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("note: {e}");
            ExitCode::FAILURE
        }
    }
}
