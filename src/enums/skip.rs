/// What [`skip`](crate::IteratorOptions::skip) skips for an iterator type, in bytes or in items
///
/// Byte and chunk iterators skip bytes, which are added to the start offset. Line and segment
/// iterators skip whole items after the read starts
///
/// # Examples
///
/// ```
/// use preader::SkipPlan;
///
/// let plan = SkipPlan::Items {
///     count: 2,
///     boundary: Some(b'\n'),
/// };
///
/// assert_eq!(plan.counts(), (0, 2));
/// ```
#[derive(Clone, Copy, Debug)]
pub enum SkipPlan {
    /// Skips this many bytes; used by byte and chunk iterators
    Bytes(u64),
    /// Skips `count` items; used by line and segment iterators
    Items {
        /// Number of items to skip
        count: u64,
        /// Boundary byte of a partial first item to skip too, set when alignment is on
        boundary: Option<u8>,
    },
}

impl SkipPlan {
    /// Returns the bytes and the items to skip, as `(bytes, items)`
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::SkipPlan;
    ///
    /// assert_eq!(SkipPlan::Bytes(3).counts(), (3, 0));
    /// ```
    pub fn counts(self) -> (u64, u64) {
        match self {
            Self::Bytes(bytes) => (bytes, 0),
            Self::Items { count, .. } => (0, count),
        }
    }

    /// Returns the alignment boundary, which only an item plan can have
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::SkipPlan;
    ///
    /// assert_eq!(SkipPlan::Bytes(3).boundary(), None);
    /// ```
    pub fn boundary(self) -> Option<u8> {
        match self {
            Self::Bytes(_) => None,
            Self::Items { boundary, .. } => boundary,
        }
    }
}
