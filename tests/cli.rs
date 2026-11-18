use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
use tempfile::TempDir;

fn note(dir: &TempDir) -> Command {
    let mut c = Command::cargo_bin("note").unwrap();
    c.env("NOTE_DB", dir.path().join("sub/notes.db"));
    c
}

#[test]
fn add_search_list_delete() {
    let d = TempDir::new().unwrap();
    note(&d).args(["buy", "milk", "and", "eggs"]).assert().success().stdout(contains("Saved #1"));
    note(&d).args(["call", "the", "dentist"]).assert().success().stdout(contains("Saved #2"));
    note(&d).args(["milk", "the", "cows"]).assert().success();

    // single word
    note(&d).args(["-s", "MILK"]).assert().success()
        .stdout(contains("buy milk").and(contains("the cows")).and(contains("dentist").not()));
    // multiple words are ANDed
    note(&d).args(["-s", "milk", "eggs"]).assert().success()
        .stdout(contains("buy milk").and(contains("cows").not()));
    // prefix
    note(&d).args(["-s", "dent*"]).assert().success().stdout(contains("dentist"));
    // punctuation doesn't break the query
    note(&d).args(["-s", "can't", "\"x"]).assert().success().stdout(contains("No notes match."));

    note(&d).args(["-l", "2"]).assert().success()
        .stdout(contains("dentist").and(contains("cows")).and(contains("eggs").not()));

    note(&d).args(["-d", "2"]).assert().success().stdout(contains("Deleted #2"));
    note(&d).args(["-s", "dentist"]).assert().success().stdout(contains("No notes match."));
    note(&d).args(["-d", "2"]).assert().failure().stderr(contains("no note with id"));
}

#[test]
fn stdin_and_hyphen_words() {
    let d = TempDir::new().unwrap();
    note(&d).arg("-").write_stdin("multi\nline note\n").assert().success();
    note(&d).args(["fix", "the", "-v", "flag"]).assert().success();
    note(&d).assert().success().stdout(contains("multi").and(contains("fix the -v flag")));
    note(&d).args(["-s", "line"]).assert().success().stdout(contains("multi"));
}
