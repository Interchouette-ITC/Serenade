//! Pagination helpers for repository list methods.

/// Limit/offset page for catalog reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    /// Maximum rows to return.
    pub limit: u32,
    /// Rows to skip before the first result.
    pub offset: u32,
}

impl PageRequest {
    /// First page with the given size.
    #[must_use]
    pub const fn first(limit: u32) -> Self {
        Self { limit, offset: 0 }
    }
}
