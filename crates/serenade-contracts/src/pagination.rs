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
    ///
    /// # Examples
    ///
    /// ```
    /// use serenade_contracts::PageRequest;
    ///
    /// let page = PageRequest::first(20);
    /// assert_eq!(page.limit, 20);
    /// assert_eq!(page.offset, 0);
    /// ```
    #[must_use]
    pub const fn first(limit: u32) -> Self {
        Self { limit, offset: 0 }
    }
}
