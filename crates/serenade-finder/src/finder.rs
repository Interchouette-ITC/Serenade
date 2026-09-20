//! Finder builder and collection.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::FinderError;
use crate::glob::name_matches;

/// Entry type filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntryKind {
    /// Files and directories.
    #[default]
    Any,
    /// Regular files only.
    File,
    /// Directories only.
    Directory,
}

/// Symfony-shaped directory walker with filters.
#[derive(Debug, Clone, Default)]
pub struct Finder {
    roots: Vec<PathBuf>,
    kind: EntryKind,
    names: Vec<String>,
    min_depth: usize,
    max_depth: Option<usize>,
    ignore_dotfiles: bool,
    follow_links: bool,
}

impl Finder {
    /// Empty finder (call [`Self::in_path`] before [`Self::collect`]).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a search root directory.
    #[must_use]
    pub fn in_path(mut self, path: impl AsRef<Path>) -> Self {
        self.roots.push(path.as_ref().to_path_buf());
        self
    }

    /// Restrict to regular files.
    #[must_use]
    pub const fn files(mut self) -> Self {
        self.kind = EntryKind::File;
        self
    }

    /// Restrict to directories.
    #[must_use]
    pub const fn directories(mut self) -> Self {
        self.kind = EntryKind::Directory;
        self
    }

    /// Basename must match at least one `*` / `?` pattern.
    #[must_use]
    pub fn name(mut self, pattern: impl Into<String>) -> Self {
        self.names.push(pattern.into());
        self
    }

    /// Minimum depth relative to each root (0 = root itself).
    #[must_use]
    pub const fn min_depth(mut self, depth: usize) -> Self {
        self.min_depth = depth;
        self
    }

    /// Maximum depth relative to each root (inclusive).
    #[must_use]
    pub const fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Skip entries whose basename starts with `.`.
    #[must_use]
    pub const fn ignore_dotfiles(mut self) -> Self {
        self.ignore_dotfiles = true;
        self
    }

    /// Follow symbolic links while walking.
    #[must_use]
    pub const fn follow_links(mut self) -> Self {
        self.follow_links = true;
        self
    }

    /// Collect matching paths sorted by path string.
    ///
    /// # Errors
    ///
    /// Empty roots, invalid root, or walk I/O failure.
    pub fn collect(self) -> Result<Vec<PathBuf>, FinderError> {
        if self.roots.is_empty() {
            return Err(FinderError::EmptyRoots);
        }
        let mut out = Vec::new();
        for root in &self.roots {
            if !root.is_dir() {
                return Err(FinderError::InvalidRoot { path: root.clone() });
            }
            let mut walker = WalkDir::new(root).min_depth(self.min_depth);
            if let Some(max) = self.max_depth {
                walker = walker.max_depth(max);
            }
            if self.follow_links {
                walker = walker.follow_links(true);
            }
            for entry in walker {
                let entry = entry.map_err(|err| {
                    let path = err.path().map_or_else(|| root.clone(), Path::to_path_buf);
                    let source = err.into_io_error().unwrap_or_else(|| {
                        std::io::Error::other("walkdir error without io source")
                    });
                    FinderError::Walk { path, source }
                })?;
                let path = entry.path();
                let file_type = entry.file_type();
                if self.ignore_dotfiles {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if name.starts_with('.') && path != root.as_path() {
                            continue;
                        }
                    }
                }
                let ok_kind = match self.kind {
                    EntryKind::Any => true,
                    EntryKind::File => file_type.is_file(),
                    EntryKind::Directory => file_type.is_dir(),
                };
                if !ok_kind {
                    continue;
                }
                if !self.names.is_empty() {
                    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                        continue;
                    };
                    if !self.names.iter().any(|pat| name_matches(pat, name)) {
                        continue;
                    }
                }
                out.push(path.to_path_buf());
            }
        }
        out.sort_by(|a, b| cmp_paths(a, b));
        Ok(out)
    }
}

fn cmp_paths(a: &Path, b: &Path) -> Ordering {
    a.as_os_str().cmp(b.as_os_str())
}
