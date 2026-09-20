use std::fs;
use std::path::{Path, PathBuf};

use crate::{Finder, FinderError, name_matches};

fn tree() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "serenade-finder-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::create_dir_all(dir.join("nested")).expect("mkdir");
    fs::write(dir.join("a.rs"), b"a").expect("a");
    fs::write(dir.join("b.txt"), b"b").expect("b");
    fs::write(dir.join("nested/c.rs"), b"c").expect("c");
    fs::write(dir.join(".hidden"), b"h").expect("hidden");
    dir
}

#[test]
fn finds_rs_files_sorted() {
    let dir = tree();
    let found = Finder::new()
        .in_path(&dir)
        .files()
        .name("*.rs")
        .collect()
        .expect("collect");
    let names: Vec<_> = found
        .iter()
        .filter_map(|p| p.file_name().and_then(|s| s.to_str()))
        .collect();
    assert_eq!(names, ["a.rs", "c.rs"]);
}

#[test]
fn directories_only_and_depth() {
    let dir = tree();
    let dirs = Finder::new()
        .in_path(&dir)
        .directories()
        .min_depth(1)
        .max_depth(1)
        .collect()
        .expect("dirs");
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].file_name().and_then(|s| s.to_str()), Some("nested"));
}

#[test]
fn ignore_dotfiles() {
    let dir = tree();
    let all = Finder::new()
        .in_path(&dir)
        .files()
        .max_depth(1)
        .collect()
        .expect("all");
    assert!(all.iter().any(|p| ends_with(p, ".hidden")));

    let filtered = Finder::new()
        .in_path(&dir)
        .files()
        .max_depth(1)
        .ignore_dotfiles()
        .collect()
        .expect("filtered");
    assert!(!filtered.iter().any(|p| ends_with(p, ".hidden")));
}

#[test]
fn empty_roots_and_invalid_root() {
    assert_eq!(
        Finder::new().collect().expect_err("empty"),
        FinderError::EmptyRoots
    );
    let missing = PathBuf::from("/definitely/missing/serenade-finder-root");
    assert_eq!(
        Finder::new().in_path(&missing).collect().expect_err("bad"),
        FinderError::InvalidRoot { path: missing }
    );
}

#[test]
fn name_matches_exported() {
    assert!(name_matches("*.toml", "Cargo.toml"));
    assert!(!name_matches("*.toml", "Cargo.lock"));
}

#[test]
fn version_is_nonzero() {
    assert_ne!(crate::version(), "");
}

#[test]
fn finder_error_eq() {
    assert_eq!(FinderError::EmptyRoots, FinderError::EmptyRoots);
    assert_ne!(
        FinderError::EmptyRoots,
        FinderError::InvalidRoot {
            path: PathBuf::from("/x"),
        }
    );
    let walk = FinderError::Walk {
        path: PathBuf::from("/w"),
        source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    };
    assert_eq!(
        walk,
        FinderError::Walk {
            path: PathBuf::from("/w"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        }
    );
    assert_ne!(
        walk,
        FinderError::Walk {
            path: PathBuf::from("/w"),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        }
    );
}

#[test]
fn any_kind_and_multiple_name_patterns() {
    let dir = tree();
    let found = Finder::new()
        .in_path(&dir)
        .name("*.rs")
        .name("*.txt")
        .max_depth(1)
        .collect()
        .expect("any");
    let names: Vec<_> = found
        .iter()
        .filter_map(|p| p.file_name().and_then(|s| s.to_str()))
        .collect();
    assert!(names.contains(&"a.rs"));
    assert!(names.contains(&"b.txt"));
}

#[test]
fn follow_links_flag_runs() {
    let dir = tree();
    let found = Finder::new()
        .in_path(&dir)
        .files()
        .follow_links()
        .name("*.rs")
        .collect()
        .expect("follow");
    assert_ne!(found.len(), 0);
}

fn ends_with(path: &Path, name: &str) -> bool {
    path.file_name().and_then(|s| s.to_str()) == Some(name)
}
