use crate::Result;

/// Reads items without allocating, by borrowing them from the iterator's buffer
///
/// [`PReaderIterator`](crate::PReaderIterator) implements [`Iterator`] on top of this trait
/// by converting each [`Borrowed`](IteratorRead::Borrowed) item into its
/// [`Owned`](IteratorRead::Owned) form. Call [`read`](IteratorRead::read) instead of
/// [`next`](Iterator::next) to avoid one allocation per item. The trait is implemented for the
/// four iterator types and is meant to be used, not implemented
pub trait IteratorRead {
    /// Item borrowed from the iterator, valid until the next read
    type Borrowed<'a>
    where
        Self: 'a;
    /// Owned form of the item, yielded by [`Iterator::next`]
    type Owned: for<'a> From<Self::Borrowed<'a>>;

    /// Reads the next item as a borrowed value, or returns `None` when the read is done
    ///
    /// Progress, autosave and errors work exactly as with [`Iterator::next`]: a failed item does
    /// not end the iteration, while a failed autosave or an I/O error that read nothing does
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) when the file cannot be read,
    /// [`Error::Utf8`](crate::Error::Utf8) when a line is not valid UTF-8, and the errors of
    /// [`PReaderIterator::save`](crate::PReaderIterator::save) when an autosave fails
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, IteratorRead, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\nbar\n")?;
    ///
    /// let mut lines = reader.lines(&file).build()?;
    /// let mut total = 0;
    ///
    /// while let Some(line) = lines.read()? {
    ///     total += line.len();
    /// }
    ///
    /// assert_eq!(total, 6);
    /// # Ok::<(), preader::Error>(())
    /// ```
    fn read(&mut self) -> Result<Option<Self::Borrowed<'_>>>;
}
