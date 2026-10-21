//! Evidence store adapter (ADR-0003): immutable files under `.git/opshero/records/`.

use opshero_core::record::{self, Record};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// What a prune did (or, for a dry run, would do).
#[derive(Debug, PartialEq, Eq)]
pub struct PruneReport {
    pub total: usize,
    pub kept: usize,
    /// Records kept only because the caller protected them.
    pub protected: usize,
    pub removed: usize,
}

pub struct Store {
    dir: PathBuf,
    /// Records fetched from published evidence. Kept apart so that they can never be
    /// mistaken for local ones, and capped at assurance A1 wherever they are read.
    imported_dir: PathBuf,
    head_file: PathBuf,
}

/// Upper bounds on what we are willing to read: hostile or corrupted files
/// must produce a bounded, explicit note, never a crash or a hang.
const MAX_RECORD_BYTES: u64 = 256 * 1024;
const MAX_RECORDS: usize = 20_000;

impl Store {
    pub fn open(git_dir: &Path) -> Self {
        let base = git_dir.join("opshero");
        Self {
            dir: base.join("records"),
            imported_dir: base.join("imported"),
            head_file: base.join("HEAD"),
        }
    }

    /// Current chain head (id of the last record written), if any.
    pub fn head(&self) -> Option<String> {
        let text = fs::read_to_string(&self.head_file).ok()?;
        let id = text.trim();
        (id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit())).then(|| id.to_owned())
    }

    /// Persist a record atomically: write a temp file, then link it into place.
    /// Records are immutable; an existing file with the same id is left as is.
    pub fn append(&self, rec: &Record) -> Result<(), String> {
        fs::create_dir_all(&self.dir).map_err(|e| format!("cannot create store: {e}"))?;
        let target = self.dir.join(format!("{}.json", rec.id));
        if !target.exists() {
            let tmp = self
                .dir
                .join(format!(".{}.tmp-{}", rec.id, std::process::id()));
            let body = serde_json::to_string(rec).map_err(|e| e.to_string())?;
            {
                let mut f = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&tmp)
                    .map_err(|e| format!("cannot write record: {e}"))?;
                f.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
                f.sync_all().map_err(|e| e.to_string())?;
            }
            fs::rename(&tmp, &target).map_err(|e| format!("cannot finalize record: {e}"))?;
        }
        // Atomic: a reader must never see an empty or half-written head.
        let tmp = self
            .head_file
            .with_extension(format!("tmp-{}", std::process::id()));
        fs::write(&tmp, format!("{}\n", rec.id))
            .map_err(|e| format!("cannot update chain head: {e}"))?;
        fs::rename(&tmp, &self.head_file).map_err(|e| format!("cannot update chain head: {e}"))
    }

    /// Add a record that came from published evidence. It goes to its own directory
    /// and is read back with its assurance capped at A1: what a publisher claims about
    /// itself proves nothing. Returns whether the file was newly written.
    pub fn import(&self, rec: &Record) -> Result<bool, String> {
        fs::create_dir_all(&self.imported_dir).map_err(|e| format!("cannot create store: {e}"))?;
        let target = self.imported_dir.join(format!("{}.json", rec.id));
        if target.exists() {
            return Ok(false);
        }
        let tmp = self
            .imported_dir
            .join(format!(".{}.import-{}", rec.id, std::process::id()));
        let body = serde_json::to_string(rec).map_err(|e| e.to_string())?;
        fs::write(&tmp, body).map_err(|e| format!("cannot write record: {e}"))?;
        fs::rename(&tmp, &target).map_err(|e| format!("cannot finalize record: {e}"))?;
        Ok(true)
    }

    /// Number of record files (cheap: one directory listing, no parsing).
    pub fn count(&self) -> usize {
        fs::read_dir(&self.dir)
            .map(|d| {
                d.filter_map(|e| e.ok())
                    .filter(|e| {
                        let n = e.file_name();
                        let n = n.to_string_lossy();
                        !n.starts_with('.') && n.ends_with(".json")
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    /// Keep the `keep` newest records (by position in the hash chain) and every
    /// record `protect` selects; delete the rest. Only records that pass the
    /// integrity check are ever deleted: corrupt, foreign and temporary files
    /// are left alone. Evidence is advisory local history (ADR-0003); a deleted
    /// record can only turn a verdict from PASS into UNKNOWN, never the reverse.
    pub fn prune(
        &self,
        keep: usize,
        dry_run: bool,
        protect: &dyn Fn(&Record) -> bool,
    ) -> Result<PruneReport, String> {
        let (records, _notes) = self.load();
        let total = records.len();
        let mut report = PruneReport {
            total,
            kept: total.min(keep),
            protected: 0,
            removed: 0,
        };
        for r in opshero_core::verify::newest_first(&records)
            .into_iter()
            .skip(keep)
        {
            if protect(r) {
                report.kept += 1;
                report.protected += 1;
                continue;
            }
            if !dry_run {
                for d in [&self.dir, &self.imported_dir] {
                    match fs::remove_file(d.join(format!("{}.json", r.id))) {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(format!("cannot delete a record: {e}")),
                    }
                }
            }
            report.removed += 1;
        }
        Ok(report)
    }

    /// Load every record, keeping only those whose content hash matches their
    /// id. Rejected files are reported in the returned notes.
    pub fn load(&self) -> (Vec<Record>, Vec<String>) {
        let (mut records, mut notes) = self.load_dir(&self.dir);
        let (imported, imported_notes) = self.load_dir(&self.imported_dir);
        notes.extend(imported_notes);
        let local: std::collections::HashSet<String> =
            records.iter().map(|r| r.id.clone()).collect();
        for mut rec in imported {
            if local.contains(&rec.id) {
                continue; // we hold the original ourselves
            }
            // A published record is only ever as good as A1, whatever it claims.
            if matches!(
                opshero_core::policy::Assurance::parse(&rec.body.recorder.assurance),
                Some(opshero_core::policy::Assurance::A2 | opshero_core::policy::Assurance::A3)
            ) {
                rec.body.recorder.assurance = "A1".to_owned();
            }
            records.push(rec);
        }
        (records, notes)
    }

    fn load_dir(&self, dir: &Path) -> (Vec<Record>, Vec<String>) {
        let mut records = Vec::new();
        let mut notes = Vec::new();
        let Ok(entries) = fs::read_dir(dir) else {
            return (records, notes);
        };
        let mut names: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();
        names.sort();
        for (i, name) in names.iter().enumerate() {
            let name_s = name.to_string_lossy();
            if name_s.starts_with('.') || !name_s.ends_with(".json") {
                continue;
            }
            if i >= MAX_RECORDS {
                notes.push(format!(
                    "more than {MAX_RECORDS} records: the rest were ignored"
                ));
                break;
            }
            let path = dir.join(name);
            match fs::metadata(&path) {
                Ok(m) if m.len() > MAX_RECORD_BYTES => {
                    notes.push(format!(
                        "{name_s}: ignored (file larger than {MAX_RECORD_BYTES} bytes)"
                    ));
                    continue;
                }
                Err(e) => {
                    notes.push(format!("{name_s}: unreadable ({e})"));
                    continue;
                }
                _ => {}
            }
            let raw = match fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| {
                    serde_json::from_str::<serde_json::Value>(&t).map_err(|e| e.to_string())
                }) {
                Ok(v) => v,
                Err(e) => {
                    notes.push(format!("{name_s}: ignored ({e})"));
                    continue;
                }
            };
            match record::parse_verified(&raw) {
                Ok(rec) => {
                    if format!("{}.json", rec.id) != name_s {
                        notes.push(format!(
                            "{name_s}: ignored (file name does not match its id)"
                        ));
                    } else {
                        records.push(rec);
                    }
                }
                Err(e) => notes.push(format!("{name_s}: ignored ({e})")),
            }
        }
        (records, notes)
    }
}
