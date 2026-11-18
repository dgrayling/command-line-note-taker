# note

Jot down notes from the terminal and search them later.

## Install

```sh
cargo install --path .
```

This puts `note` in `~/.cargo/bin`, which is on your PATH if you installed Rust with rustup.

## Usage

```sh
note call the dentist on friday   # save a note
note                              # show the 10 most recent notes
note -s dentist                   # search (all words must match, case-insensitive)
note -s dent*                     # prefix search
note -l 50                        # list the last 50 notes (default 20)
note -d 12                        # delete note #12
echo "longer text" | note -       # read a note from stdin
```

Quote notes containing shell characters like `'`, `?`, `*`, `#` or `;`:
`note "don't forget: what's next?"`

## Storage

Notes live in a SQLite database at `~/.local/share/note/notes.db`, with full-text search provided by FTS5.
Set `NOTE_DB=/some/path.db` to use a different file.
