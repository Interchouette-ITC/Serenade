# Filesystem

Path helpers live in **`serenade-filesystem`** ([#241](https://github.com/Interchouette-ITC/Serenade/issues/241)).

Symfony Filesystem shaped: mkdir, dump, mirror, temp. Thin wrappers over `std::fs` + `tempfile`.

## API

| Function | Role |
| --- | --- |
| `exists` / `is_file` / `is_dir` | Presence checks |
| `mkdir` | Create directory + parents (`mkdir -p`) |
| `remove` / `remove_tree` | Delete file or empty dir / recursive tree |
| `rename` | Move path (creates destination parents) |
| `dump_file` / `append_to_file` | Write / append bytes (creates parents) |
| `read` / `read_to_string` | Read whole file |
| `touch` | Create empty file or refresh existing |
| `copy` / `mirror` | Copy file / recursive directory tree |
| `temp_dir` / `temp_file` / `*_in` | Temporary paths (`tempfile` crate) |

## Example

```rust
use serenade_filesystem::{dump_file, mkdir, mirror, read_to_string, temp_dir};

let dir = temp_dir()?;
let src = dir.path().join("src");
mkdir(src.join("nested"))?;
dump_file(src.join("a.txt"), b"hi")?;
let dst = dir.path().join("dst");
mirror(&src, &dst)?;
assert_eq!(read_to_string(dst.join("a.txt"))?, "hi");
```

## Limits

- Symlinks under `mirror` are skipped (not followed, not copied)
- `remove` on a non-empty directory fails; use `remove_tree`
- Not a VFS / remote filesystem abstraction
- Process spawn and Finder/glob are separate crates in this wave

## Related

- [KERNEL.md](KERNEL.md) - component index
- [WORKFLOW.md](WORKFLOW.md) - state machines (unrelated to OS paths)
