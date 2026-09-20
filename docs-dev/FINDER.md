# Finder

Directory walking helpers live in **`serenade-finder`** ([#245](https://github.com/Interchouette-ITC/Serenade/issues/245)).

Symfony Finder shaped: roots, files/directories, name globs, depth, ignore dotfiles.

## API

| Piece | Role |
| --- | --- |
| `Finder::new` / `in_path` | Search roots |
| `files` / `directories` | Type filter |
| `name` | Basename `*` / `?` glob (OR across patterns) |
| `min_depth` / `max_depth` | Walk depth relative to each root |
| `ignore_dotfiles` | Skip basenames starting with `.` |
| `follow_links` | Follow symlinks |
| `collect` | Sorted `Vec<PathBuf>` |

## Example

```rust
use serenade_finder::Finder;

let paths = Finder::new()
    .in_path("crates")
    .files()
    .name("*.rs")
    .ignore_dotfiles()
    .collect()?;
```

## Limits

- Name match is basename-only (not full path globs)
- No `.gitignore` / VCS exclusion
- No content search
- Built on `walkdir`

## Related

- [KERNEL.md](KERNEL.md) - component index
- [FILESYSTEM.md](FILESYSTEM.md) - path helpers
- [PROCESS.md](PROCESS.md) - child processes
