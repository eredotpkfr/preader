use crate::{Error, Result, enums::skip::SkipPlan, macros::ensure, types::window::Window};

/// Byte range, skip count and item limit that select which part of a file an iterator reads
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "preader", eq, frozen, from_py_object, get_all)
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IteratorOptions {
    /// Byte offset where reading begins
    pub start: u64,
    /// Byte offset at which no new item starts, clamped to the recorded file size
    pub end: u64,
    /// Number of items skipped after `start`, counted in bytes by byte iterators
    pub skip: u64,
    /// Maximum number of items one iterator yields
    pub limit: u64,
}

impl Default for IteratorOptions {
    /// Creates options that read the whole file, with no skip and no limit
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::IteratorOptions;
    ///
    /// let options = IteratorOptions::default();
    ///
    /// assert_eq!(options.end, u64::MAX);
    /// ```
    fn default() -> Self {
        Self {
            start: 0,
            end: u64::MAX,
            skip: 0,
            limit: u64::MAX,
        }
    }
}

impl IteratorOptions {
    /// Resolves the options into the window one read covers
    ///
    /// The end is clamped to the recorded file `size`, the start moves past the bytes of the
    /// `skip` plan up to that end, and a resumed `position` further than that start wins. Items
    /// are skipped only when `position` is at or before that start and before the end. The
    /// [`skip`](Self::skip) field is not read, the caller passes it in the plan
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{IteratorOptions, SkipPlan};
    ///
    /// let options = IteratorOptions {
    ///     start: 2,
    ///     ..IteratorOptions::default()
    /// };
    /// let window = options.window(0, 10, SkipPlan::Bytes(3));
    ///
    /// assert_eq!(window.position, 5);
    /// ```
    #[cfg_attr(not(feature = "testing"), doc(hidden))]
    pub fn window(&self, position: u64, size: u64, skip: SkipPlan) -> Window {
        let (bytes, items) = skip.counts();
        let end = self.end.min(size);
        let start = self.start.saturating_add(bytes).min(end);

        Window {
            position: position.max(start),
            end,
            limit: self.limit,
            skipping: (position <= start && position < end).then_some(items),
            boundary: skip.boundary(),
        }
    }

    /// Checks that the options describe a valid range
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidRange`] when [`start`] is greater than [`end`]
    ///
    /// [`start`]: Self::start
    /// [`end`]: Self::end
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(self.start <= self.end, Error::InvalidRange);

        Ok(())
    }
}
