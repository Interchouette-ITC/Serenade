use std::path::PathBuf;

use crate::{
    FilesystemError, append_to_file, copy, dump_file, exists, is_dir, is_file, mirror, mkdir, read,
    read_to_string, remove, remove_tree, rename, temp_dir, temp_dir_in, temp_file, temp_file_in,
    touch,
};

#[test]
fn dump_read_append_and_mkdir() {
    let dir = temp_dir().expect("temp");
    let nested = dir.path().join("x/y");
    mkdir(&nested).expect("mkdir");
    assert!(is_dir(&nested));

    let file = nested.join("f.txt");
    dump_file(&file, b"hello").expect("dump");
    assert!(is_file(&file));
    assert_eq!(read_to_string(&file).expect("read"), "hello");
    assert_eq!(read(&file).expect("bytes"), b"hello");

    append_to_file(&file, b" world").expect("append");
    assert_eq!(read_to_string(&file).expect("read2"), "hello world");
}

#[test]
fn touch_creates_and_is_idempotent() {
    let dir = temp_dir().expect("temp");
    let file = dir.path().join("touched");
    touch(&file).expect("create");
    assert!(exists(&file));
    touch(&file).expect("again");
    assert!(is_file(&file));
}

#[test]
fn remove_file_and_empty_dir() {
    let dir = temp_dir().expect("temp");
    let file = dir.path().join("gone.txt");
    dump_file(&file, b"x").expect("dump");
    remove(&file).expect("remove file");
    assert!(!exists(&file));
    remove(&file).expect("noop");

    let empty = dir.path().join("empty");
    mkdir(&empty).expect("mkdir");
    remove(&empty).expect("remove dir");
    assert!(!exists(&empty));
}

#[test]
fn remove_tree_deletes_nested() {
    let dir = temp_dir().expect("temp");
    let nested = dir.path().join("tree/a");
    mkdir(&nested).expect("mkdir");
    dump_file(nested.join("f"), b"1").expect("dump");
    remove_tree(dir.path().join("tree")).expect("tree");
    assert!(!exists(dir.path().join("tree")));
}

#[test]
fn rename_and_copy() {
    let dir = temp_dir().expect("temp");
    let a = dir.path().join("a.txt");
    let b = dir.path().join("sub/b.txt");
    dump_file(&a, b"data").expect("dump");
    rename(&a, &b).expect("rename");
    assert!(!exists(&a));
    assert_eq!(read_to_string(&b).expect("read"), "data");

    let c = dir.path().join("c.txt");
    copy(&b, &c).expect("copy");
    assert_eq!(read_to_string(&c).expect("read"), "data");
}

#[test]
fn mirror_copies_tree() {
    let dir = temp_dir().expect("temp");
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    mkdir(src.join("nested")).expect("mkdir");
    dump_file(src.join("root.txt"), b"r").expect("dump");
    dump_file(src.join("nested/child.txt"), b"c").expect("dump");

    mirror(&src, &dst).expect("mirror");
    assert_eq!(read_to_string(dst.join("root.txt")).expect("r"), "r");
    assert_eq!(
        read_to_string(dst.join("nested/child.txt")).expect("c"),
        "c"
    );
}

#[test]
fn read_missing_and_not_file() {
    let dir = temp_dir().expect("temp");
    let missing = dir.path().join("nope");
    assert_eq!(
        read(&missing).expect_err("missing"),
        FilesystemError::NotFound {
            path: PathBuf::from(&missing),
        }
    );
    assert_eq!(
        read(dir.path()).expect_err("dir"),
        FilesystemError::NotFile {
            path: PathBuf::from(dir.path()),
        }
    );
}

#[test]
fn mkdir_rejects_existing_file() {
    let dir = temp_dir().expect("temp");
    let file = dir.path().join("f");
    dump_file(&file, b"x").expect("dump");
    assert_eq!(
        mkdir(&file).expect_err("not dir"),
        FilesystemError::NotDirectory {
            path: PathBuf::from(&file),
        }
    );
}

#[test]
fn copy_requires_file() {
    let dir = temp_dir().expect("temp");
    let err = copy(dir.path(), dir.path().join("x")).expect_err("not file");
    assert!(matches!(err, FilesystemError::NotFile { .. }));
    let missing = dir.path().join("missing");
    assert_eq!(
        copy(&missing, dir.path().join("y")).expect_err("nf"),
        FilesystemError::NotFound {
            path: PathBuf::from(&missing),
        }
    );
}

#[test]
fn mirror_requires_directory() {
    let dir = temp_dir().expect("temp");
    let file = dir.path().join("f");
    dump_file(&file, b"x").expect("dump");
    let err = mirror(&file, dir.path().join("out")).expect_err("not dir");
    assert!(matches!(err, FilesystemError::NotDirectory { .. }));
    let missing = dir.path().join("missing");
    assert_eq!(
        mirror(&missing, dir.path().join("out")).expect_err("nf"),
        FilesystemError::NotFound {
            path: PathBuf::from(&missing),
        }
    );
}

#[test]
fn rename_missing_errors() {
    let dir = temp_dir().expect("temp");
    let missing = dir.path().join("gone");
    assert_eq!(
        rename(&missing, dir.path().join("x")).expect_err("nf"),
        FilesystemError::NotFound {
            path: PathBuf::from(&missing),
        }
    );
}

#[test]
fn temp_helpers_under_parent() {
    let parent = temp_dir().expect("parent");
    let child = temp_dir_in(parent.path()).expect("child");
    assert!(is_dir(child.path()));
    let file = temp_file_in(parent.path(), "pre-", ".tmp").expect("file");
    assert!(exists(file.path()));
    let named = temp_file().expect("named");
    assert!(exists(named.path()));
}

#[test]
fn version_is_nonzero() {
    assert_ne!(crate::version(), "");
}

#[test]
fn read_to_string_missing_and_not_file() {
    let dir = temp_dir().expect("temp");
    let missing = dir.path().join("nope");
    assert_eq!(
        read_to_string(&missing).expect_err("missing"),
        FilesystemError::NotFound {
            path: PathBuf::from(&missing),
        }
    );
    assert_eq!(
        read_to_string(dir.path()).expect_err("dir"),
        FilesystemError::NotFile {
            path: PathBuf::from(dir.path()),
        }
    );
}

#[test]
fn remove_tree_noop_and_single_file() {
    let dir = temp_dir().expect("temp");
    let missing = dir.path().join("absent");
    remove_tree(&missing).expect("noop");

    let file = dir.path().join("alone.txt");
    dump_file(&file, b"x").expect("dump");
    remove_tree(&file).expect("file");
    assert!(!exists(&file));
}

#[test]
fn remove_nonempty_dir_is_io_error() {
    let dir = temp_dir().expect("temp");
    let nested = dir.path().join("full");
    mkdir(&nested).expect("mkdir");
    dump_file(nested.join("f"), b"1").expect("dump");
    let err = remove(&nested).expect_err("nonempty");
    assert!(matches!(err, FilesystemError::Io { op: "remove", .. }));
}

#[test]
fn filesystem_error_eq_covers_io_and_mismatch() {
    let a = FilesystemError::io(
        "dump_file",
        "a.txt",
        std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    );
    let b = FilesystemError::io(
        "dump_file",
        "a.txt",
        std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    );
    let c = FilesystemError::io(
        "dump_file",
        "a.txt",
        std::io::Error::from(std::io::ErrorKind::NotFound),
    );
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(
        a,
        FilesystemError::NotFound {
            path: PathBuf::from("a.txt"),
        }
    );
    assert_eq!(
        FilesystemError::NotDirectory {
            path: PathBuf::from("d"),
        },
        FilesystemError::NotDirectory {
            path: PathBuf::from("d"),
        }
    );
}

#[test]
fn bare_relative_paths_skip_parent_mkdir() {
    let dir = temp_dir().expect("temp");
    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("chdir");
    let result = (|| {
        dump_file("bare.txt", b"hi")?;
        append_to_file("bare.txt", b"!")?;
        touch("touched.txt")?;
        copy("bare.txt", "copy.txt")?;
        rename("copy.txt", "renamed.txt")?;
        assert_eq!(read_to_string("bare.txt")?, "hi!");
        assert_eq!(read_to_string("renamed.txt")?, "hi!");
        assert!(is_file("touched.txt"));
        Ok::<(), FilesystemError>(())
    })();
    std::env::set_current_dir(&prev).expect("restore cwd");
    result.expect("bare paths");
}

#[cfg(unix)]
#[test]
fn mirror_skips_symlinks() {
    let dir = temp_dir().expect("temp");
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    mkdir(&src).expect("mkdir");
    dump_file(src.join("real.txt"), b"ok").expect("dump");
    std::os::unix::fs::symlink("real.txt", src.join("link.txt")).expect("symlink");
    mirror(&src, &dst).expect("mirror");
    assert_eq!(read_to_string(dst.join("real.txt")).expect("real"), "ok");
    assert!(!exists(dst.join("link.txt")));
}

#[test]
fn mkdir_noop_when_already_directory() {
    let dir = temp_dir().expect("temp");
    mkdir(dir.path()).expect("first");
    mkdir(dir.path()).expect("second");
}
