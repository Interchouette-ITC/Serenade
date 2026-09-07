//! SQLite-backed feed store (posts, likes, categories, moderated comments).

#![allow(clippy::significant_drop_tightening)]

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, params};

/// Moderation state for a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStatus {
    /// Waiting for owner approval.
    Pending,
    /// Visible on the public feed.
    Approved,
}

impl CommentStatus {
    fn parse(raw: &str) -> Self {
        if raw == "approved" {
            Self::Approved
        } else {
            Self::Pending
        }
    }
}

/// One wall post.
#[derive(Debug, Clone)]
pub struct Post {
    /// Stable id.
    pub id: u64,
    /// Sanitized rich HTML (Quill); never raw untrusted markup.
    pub body: String,
    /// Optional embed / media URL (validated against allowlist before store).
    pub embed_url: Option<String>,
    /// Optional inline image as a data URL (size-capped by the app).
    pub image_data: Option<String>,
    /// Category label from the admin-managed list.
    pub category: String,
    /// Public like count.
    pub likes: u64,
    /// UTC timestamp from SQLite `datetime('now')` (display-ready).
    pub created_at: String,
}

/// One comment on a post.
#[derive(Debug, Clone)]
pub struct Comment {
    /// Stable id.
    pub id: u64,
    /// Parent post.
    pub post_id: u64,
    /// Escaped later at render time.
    pub body: String,
    /// Moderation state.
    pub status: CommentStatus,
}

const DEFAULT_CATEGORIES: &[&str] = &["Work", "Life", "Family", "Travel", "Friends", "Ideas"];

/// Shared SQLite store.
#[derive(Clone)]
pub struct FeedStore {
    conn: Arc<Mutex<Connection>>,
}

/// Fields for creating a post.
pub struct NewPost {
    /// Sanitized HTML body.
    pub body: String,
    /// Optional media URL.
    pub embed_url: Option<String>,
    /// Optional data-URL image.
    pub image_data: Option<String>,
    /// Category label.
    pub category: String,
}

fn sql_id(id: u64) -> i64 {
    i64::try_from(id).unwrap_or(i64::MAX)
}

fn rust_id(id: i64) -> u64 {
    u64::try_from(id).unwrap_or(0)
}

impl FeedStore {
    /// Opens (or creates) the SQLite database at `path` and migrates schema.
    ///
    /// # Panics
    ///
    /// Panics when the database cannot be opened or migrated (demo fail-fast).
    #[must_use]
    pub fn open(path: impl AsRef<Path>) -> Self {
        let conn = Connection::open(path.as_ref()).expect("open MyFeed sqlite");
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS posts (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              body TEXT NOT NULL,
              embed_url TEXT,
              image_data TEXT,
              category TEXT NOT NULL,
              likes INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS comments (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              post_id INTEGER NOT NULL,
              body TEXT NOT NULL,
              status TEXT NOT NULL CHECK (status IN ('pending', 'approved')),
              FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS categories (
              name TEXT PRIMARY KEY
            );
            ",
        )
        .expect("migrate MyFeed sqlite");
        ensure_created_at_column(&conn);
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.ensure_default_categories();
        store
    }

    /// In-memory SQLite (tests).
    #[cfg(test)]
    #[must_use]
    pub fn open_memory() -> Self {
        Self::open(":memory:")
    }

    fn ensure_default_categories(&self) {
        let conn = self.conn.lock().expect("feed lock");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
            .unwrap_or(0);
        if count > 0 {
            return;
        }
        for name in DEFAULT_CATEGORIES {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO categories (name) VALUES (?1)",
                params![*name],
            );
        }
    }

    /// Newest posts first.
    #[must_use]
    pub fn posts(&self) -> Vec<Post> {
        let conn = self.conn.lock().expect("feed lock");
        let mut stmt = conn
            .prepare(
                "SELECT id, body, embed_url, image_data, category, likes, created_at
                 FROM posts ORDER BY id DESC",
            )
            .expect("prepare posts");
        let rows = stmt.query_map([], map_post).expect("query posts");
        rows.filter_map(Result::ok).collect()
    }

    /// Loads one post by id.
    #[must_use]
    pub fn get_post(&self, id: u64) -> Option<Post> {
        self.conn
            .lock()
            .expect("feed lock")
            .query_row(
                "SELECT id, body, embed_url, image_data, category, likes, created_at
                 FROM posts WHERE id = ?1",
                params![sql_id(id)],
                map_post,
            )
            .optional()
            .ok()
            .flatten()
    }

    /// Category labels for the composer + admin.
    #[must_use]
    pub fn categories(&self) -> Vec<String> {
        let conn = self.conn.lock().expect("feed lock");
        let mut stmt = conn
            .prepare("SELECT name FROM categories ORDER BY name COLLATE NOCASE")
            .expect("prepare categories");
        let rows = stmt
            .query_map([], |row| row.get(0))
            .expect("query categories");
        rows.filter_map(Result::ok).collect()
    }

    /// Adds a category (max 40 chars, unique case-insensitive).
    pub fn add_category(&self, name: &str) -> Result<(), &'static str> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Category name is required.");
        }
        let name: String = name.chars().take(40).collect();
        let conn = self.conn.lock().expect("feed lock");
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM categories WHERE name = ?1 COLLATE NOCASE LIMIT 1",
                params![name],
                |_| Ok(true),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or(false);
        if exists {
            return Err("Category already exists.");
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
            .unwrap_or(0);
        if count >= 32 {
            return Err("Too many categories.");
        }
        conn.execute("INSERT INTO categories (name) VALUES (?1)", params![name])
            .map_err(|_| "Could not add category.")?;
        Ok(())
    }

    /// Removes a category by exact name.
    pub fn remove_category(&self, name: &str) -> bool {
        self.conn
            .lock()
            .expect("feed lock")
            .execute("DELETE FROM categories WHERE name = ?1", params![name])
            .is_ok_and(|n| n > 0)
    }

    /// True when `name` is in the managed list.
    #[must_use]
    pub fn has_category(&self, name: &str) -> bool {
        self.conn
            .lock()
            .expect("feed lock")
            .query_row(
                "SELECT 1 FROM categories WHERE name = ?1 LIMIT 1",
                params![name],
                |_| Ok(true),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or(false)
    }

    /// Approved comments for `post_id`.
    #[must_use]
    pub fn approved_comments(&self, post_id: u64) -> Vec<Comment> {
        self.comments_where(post_id, CommentStatus::Approved)
    }

    /// Pending comments (admin queue), newest first.
    #[must_use]
    pub fn pending_comments(&self) -> Vec<Comment> {
        let conn = self.conn.lock().expect("feed lock");
        let mut stmt = conn
            .prepare(
                "SELECT id, post_id, body, status FROM comments
                 WHERE status = 'pending' ORDER BY id DESC",
            )
            .expect("prepare pending");
        let rows = stmt.query_map([], map_comment).expect("query pending");
        rows.filter_map(Result::ok).collect()
    }

    fn comments_where(&self, post_id: u64, status: CommentStatus) -> Vec<Comment> {
        let status_sql = match status {
            CommentStatus::Pending => "pending",
            CommentStatus::Approved => "approved",
        };
        let conn = self.conn.lock().expect("feed lock");
        let mut stmt = conn
            .prepare(
                "SELECT id, post_id, body, status FROM comments
                 WHERE post_id = ?1 AND status = ?2 ORDER BY id ASC",
            )
            .expect("prepare comments");
        let rows = stmt
            .query_map(params![sql_id(post_id), status_sql], map_comment)
            .expect("query comments");
        rows.filter_map(Result::ok)
            .filter(|comment| comment.status == status)
            .collect()
    }

    /// Adds a post; media already allowlist / size-checked by the caller.
    pub fn add_post(&self, new: NewPost) -> Post {
        let conn = self.conn.lock().expect("feed lock");
        conn.execute(
            "INSERT INTO posts (body, embed_url, image_data, category, likes, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, datetime('now'))",
            params![new.body, new.embed_url, new.image_data, new.category],
        )
        .expect("insert post");
        let id = rust_id(conn.last_insert_rowid());
        let created_at: String = conn
            .query_row(
                "SELECT created_at FROM posts WHERE id = ?1",
                params![sql_id(id)],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| String::new());
        drop(conn);
        Post {
            id,
            body: new.body,
            embed_url: new.embed_url,
            image_data: new.image_data,
            category: new.category,
            likes: 0,
            created_at,
        }
    }

    /// Updates post body / media / category (admin).
    pub fn update_post(&self, id: u64, new: &NewPost) -> bool {
        self.conn
            .lock()
            .expect("feed lock")
            .execute(
                "UPDATE posts SET body = ?1, embed_url = ?2, image_data = ?3, category = ?4
                 WHERE id = ?5",
                params![
                    new.body,
                    new.embed_url,
                    new.image_data,
                    new.category,
                    sql_id(id)
                ],
            )
            .is_ok_and(|n| n > 0)
    }

    /// Deletes a post and its comments (admin).
    pub fn delete_post(&self, id: u64) -> bool {
        self.conn
            .lock()
            .expect("feed lock")
            .execute("DELETE FROM posts WHERE id = ?1", params![sql_id(id)])
            .is_ok_and(|n| n > 0)
    }

    /// Increments the public like counter.
    pub fn like_post(&self, post_id: u64) -> Option<u64> {
        let conn = self.conn.lock().expect("feed lock");
        let updated = conn
            .execute(
                "UPDATE posts SET likes = likes + 1 WHERE id = ?1",
                params![sql_id(post_id)],
            )
            .ok()?;
        if updated == 0 {
            return None;
        }
        let likes = conn
            .query_row(
                "SELECT likes FROM posts WHERE id = ?1",
                params![sql_id(post_id)],
                |row| row.get::<_, i64>(0),
            )
            .ok()
            .map(rust_id);
        drop(conn);
        likes
    }

    /// Adds a pending comment.
    pub fn add_comment(&self, post_id: u64, body: String) -> Option<Comment> {
        let conn = self.conn.lock().expect("feed lock");
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM posts WHERE id = ?1",
                params![sql_id(post_id)],
                |_| Ok(true),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or(false);
        if !exists {
            return None;
        }
        conn.execute(
            "INSERT INTO comments (post_id, body, status) VALUES (?1, ?2, 'pending')",
            params![sql_id(post_id), body],
        )
        .ok()?;
        let id = rust_id(conn.last_insert_rowid());
        drop(conn);
        Some(Comment {
            id,
            post_id,
            body,
            status: CommentStatus::Pending,
        })
    }

    /// Approves a pending comment.
    pub fn approve_comment(&self, id: u64) -> bool {
        self.conn
            .lock()
            .expect("feed lock")
            .execute(
                "UPDATE comments SET status = 'approved' WHERE id = ?1",
                params![sql_id(id)],
            )
            .is_ok_and(|n| n > 0)
    }

    /// Removes a comment (reject / delete).
    pub fn remove_comment(&self, id: u64) -> bool {
        self.conn
            .lock()
            .expect("feed lock")
            .execute("DELETE FROM comments WHERE id = ?1", params![sql_id(id)])
            .is_ok_and(|n| n > 0)
    }
}

fn ensure_created_at_column(conn: &Connection) {
    let mut stmt = conn
        .prepare("PRAGMA table_info(posts)")
        .expect("pragma posts");
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("pragma map")
        .filter_map(Result::ok)
        .collect();
    if names.iter().any(|name| name == "created_at") {
        return;
    }
    conn.execute(
        "ALTER TABLE posts ADD COLUMN created_at TEXT NOT NULL DEFAULT ''",
        [],
    )
    .expect("alter created_at");
    conn.execute(
        "UPDATE posts SET created_at = datetime('now') WHERE created_at = ''",
        [],
    )
    .expect("backfill created_at");
}

fn map_post(row: &rusqlite::Row<'_>) -> rusqlite::Result<Post> {
    Ok(Post {
        id: rust_id(row.get::<_, i64>(0)?),
        body: row.get(1)?,
        embed_url: row.get(2)?,
        image_data: row.get(3)?,
        category: row.get(4)?,
        likes: rust_id(row.get::<_, i64>(5)?),
        created_at: row.get(6)?,
    })
}

fn map_comment(row: &rusqlite::Row<'_>) -> rusqlite::Result<Comment> {
    Ok(Comment {
        id: rust_id(row.get::<_, i64>(0)?),
        post_id: rust_id(row.get::<_, i64>(1)?),
        body: row.get(2)?,
        status: CommentStatus::parse(&row.get::<_, String>(3)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moderation_likes_and_categories() {
        let store = FeedStore::open_memory();
        assert!(store.categories().contains(&"Work".to_owned()));
        assert!(store.add_category("Music").is_ok());
        assert!(store.remove_category("Music"));
        let post = store.add_post(NewPost {
            body: "<p>hello</p>".into(),
            embed_url: None,
            image_data: None,
            category: "Life".into(),
        });
        assert_eq!(store.like_post(post.id), Some(1));
        let comment = store.add_comment(post.id, "nice".into()).expect("comment");
        assert_eq!(comment.status, CommentStatus::Pending);
        assert!(store.approve_comment(comment.id));
        assert_eq!(store.approved_comments(post.id).len(), 1);
        assert_eq!(
            store.approved_comments(post.id)[0].status,
            CommentStatus::Approved
        );
        assert_ne!(post.created_at, "");
        assert!(store.update_post(
            post.id,
            &NewPost {
                body: "<p>edited</p>".into(),
                embed_url: None,
                image_data: None,
                category: "Work".into(),
            }
        ));
        assert!(
            store
                .get_post(post.id)
                .is_some_and(|p| p.body.contains("edited"))
        );
        assert!(store.delete_post(post.id));
        assert!(store.get_post(post.id).is_none());
    }

    #[test]
    fn open_persists_across_reload() {
        let dir = std::env::temp_dir().join(format!("myfeed-sqlite-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("feed.sqlite");
        let _ = std::fs::remove_file(&path);
        {
            let store = FeedStore::open(&path);
            assert!(store.posts().is_empty());
            let _ = store.add_post(NewPost {
                body: "<p>kept</p>".into(),
                embed_url: None,
                image_data: None,
                category: "Life".into(),
            });
        }
        let store2 = FeedStore::open(&path);
        assert_eq!(store2.posts().len(), 1);
        assert!(store2.posts()[0].body.contains("kept"));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
