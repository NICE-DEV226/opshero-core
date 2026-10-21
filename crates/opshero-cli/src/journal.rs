//! Dogfooding journal: the file adapter (docs/validation-plan.md H1).
//!
//! Opt-in (a marker file), local (`.git/opshero/`, never committed or sent
//! anywhere) and content-free: see `opshero_core::journal` for what is stored.
//! Logging can never break the agent: callers ignore failures after a warning.

use opshero_core::journal::Event;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Refuse to read a journal bigger than this (it holds one short line per event).
const MAX_JOURNAL_BYTES: u64 = 16 * 1024 * 1024;

fn dir(git_dir: &Path) -> PathBuf {
    git_dir.join("opshero")
}

pub fn journal_path(git_dir: &Path) -> PathBuf {
    dir(git_dir).join("journal.jsonl")
}

fn marker(git_dir: &Path) -> PathBuf {
    dir(git_dir).join("journal.enabled")
}

pub fn enabled(git_dir: &Path) -> bool {
    marker(git_dir).exists()
}

pub fn enable(git_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir(git_dir))
        .map_err(|e| format!("cannot create the journal directory: {e}"))?;
    fs::write(marker(git_dir), b"").map_err(|e| format!("cannot enable the journal: {e}"))
}

/// Stop logging; `delete` also removes the journal file.
pub fn disable(git_dir: &Path, delete: bool) -> Result<(), String> {
    match fs::remove_file(marker(git_dir)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("cannot disable the journal: {e}")),
    }
    if delete {
        match fs::remove_file(journal_path(git_dir)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("cannot delete the journal: {e}")),
        }
    }
    Ok(())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Append one event if the journal is enabled. Never panics, never blocks the caller
/// for long; a failure is reported as a warning on stderr and otherwise ignored.
pub fn log(git_dir: &Path, event: &Event) {
    if !enabled(git_dir) {
        return;
    }
    if let Err(e) = append(git_dir, event) {
        eprintln!("opshero: warning: journal: {e}");
    }
}

pub fn append(git_dir: &Path, event: &Event) -> Result<(), String> {
    let mut line = event.to_line(now_secs());
    line.push('\n');
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal_path(git_dir))
        .map_err(|e| format!("cannot open {}: {e}", journal_path(git_dir).display()))?;
    // One write call of one short line: appends from concurrent hooks do not interleave.
    f.write_all(line.as_bytes())
        .map_err(|e| format!("cannot write: {e}"))
}

/// Events with their 1-based line numbers, and the number of unreadable lines.
pub fn load(git_dir: &Path) -> Result<(Vec<(usize, Event)>, usize), String> {
    let path = journal_path(git_dir);
    let meta = match fs::metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((vec![], 0)),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    if meta.len() > MAX_JOURNAL_BYTES {
        return Err(format!(
            "{} is larger than {} MiB; refusing to read it",
            path.display(),
            MAX_JOURNAL_BYTES / (1024 * 1024)
        ));
    }
    let text =
        fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut events = Vec::new();
    let mut skipped = 0;
    for (i, line) in text.lines().enumerate() {
        match Event::parse_line(line) {
            Some(e) => events.push((i + 1, e)),
            None => skipped += 1,
        }
    }
    Ok((events, skipped))
}
