//! The state of the working tree at the start of an agent turn.
//!
//! The Stop gate compares the tree now with this baseline, so a turn that changed
//! nothing (a question, a read-only investigation) is never asked for evidence,
//! and a turn that only touched files a requirement does not care about is not
//! either. One small file per session under `.git/opshero/baseline/`, keyed by a
//! one-way hash of the session id (the same key as the journal).

use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub struct Baseline {
    pub tree: String,
    /// The exclusions the tree was computed with; a tree is only comparable to
    /// another computed with the same list.
    pub exclude: Vec<String>,
}

/// Keep at most this many baselines; older ones are removed when a new one is saved.
const MAX_BASELINES: usize = 100;

fn dir(git_dir: &Path) -> PathBuf {
    git_dir.join("opshero").join("baseline")
}

fn file(git_dir: &Path, key: &str) -> PathBuf {
    dir(git_dir).join(format!("{key}.json"))
}

pub fn save(git_dir: &Path, key: &str, baseline: &Baseline) -> Result<(), String> {
    let d = dir(git_dir);
    fs::create_dir_all(&d).map_err(|e| format!("cannot create {}: {e}", d.display()))?;
    let tmp = d.join(format!(".{key}.tmp-{}", std::process::id()));
    let body = json!({"tree": baseline.tree, "exclude": baseline.exclude}).to_string();
    fs::write(&tmp, body).map_err(|e| format!("cannot write the baseline: {e}"))?;
    fs::rename(&tmp, file(git_dir, key)).map_err(|e| format!("cannot write the baseline: {e}"))?;
    prune(&d);
    Ok(())
}

pub fn load(git_dir: &Path, key: &str) -> Option<Baseline> {
    let text = fs::read_to_string(file(git_dir, key)).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    Some(Baseline {
        tree: v.get("tree")?.as_str()?.to_owned(),
        exclude: v
            .get("exclude")?
            .as_array()?
            .iter()
            .map(|e| e.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()?,
    })
}

fn prune(d: &Path) {
    let Ok(entries) = fs::read_dir(d) else { return };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    if files.len() <= MAX_BASELINES {
        return;
    }
    files.sort();
    for (_, p) in files.iter().take(files.len() - MAX_BASELINES) {
        let _ = fs::remove_file(p);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn a_saved_baseline_is_read_back_and_a_new_one_replaces_it() {
        let d = TempDir::new().unwrap();
        assert_eq!(load(d.path(), "abc"), None);
        let a = Baseline {
            tree: "t1".into(),
            exclude: vec![".claude".into()],
        };
        save(d.path(), "abc", &a).unwrap();
        assert_eq!(load(d.path(), "abc"), Some(a));
        let b = Baseline {
            tree: "t2".into(),
            exclude: vec![],
        };
        save(d.path(), "abc", &b).unwrap();
        assert_eq!(load(d.path(), "abc"), Some(b));
        assert_eq!(load(d.path(), "other"), None, "keys do not mix");
    }

    #[test]
    fn a_corrupt_baseline_is_ignored_not_trusted() {
        let d = TempDir::new().unwrap();
        save(
            d.path(),
            "k",
            &Baseline {
                tree: "t".into(),
                exclude: vec![],
            },
        )
        .unwrap();
        fs::write(file(d.path(), "k"), "not json").unwrap();
        assert_eq!(load(d.path(), "k"), None);
    }

    #[test]
    fn the_number_of_baselines_is_bounded() {
        let d = TempDir::new().unwrap();
        for i in 0..(MAX_BASELINES + 30) {
            save(
                d.path(),
                &format!("k{i:04}"),
                &Baseline {
                    tree: "t".into(),
                    exclude: vec![],
                },
            )
            .unwrap();
        }
        let n = fs::read_dir(dir(d.path())).unwrap().count();
        assert!(n <= MAX_BASELINES, "{n} baselines kept");
        assert!(
            load(d.path(), &format!("k{:04}", MAX_BASELINES + 29)).is_some(),
            "the newest survives"
        );
    }
}
