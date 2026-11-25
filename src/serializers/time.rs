/// Stores a `UtcDateTime` as whole seconds since the Unix epoch
pub mod seconds {
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};
    use time::UtcDateTime;

    /// Serializes `stamp` as an integer of whole Unix seconds, dropping any fraction
    ///
    /// # Errors
    ///
    /// Returns the serializer's error when the integer cannot be written
    pub fn serialize<S: Serializer>(stamp: &UtcDateTime, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(stamp.unix_timestamp())
    }

    /// Deserializes an integer of Unix seconds into a `UtcDateTime`
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error when the value is not an `i64` or lies outside the
    /// range of `UtcDateTime`
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<UtcDateTime, D::Error> {
        UtcDateTime::from_unix_timestamp(i64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// Stores a `UtcDateTime` as whole microseconds since the Unix epoch
pub mod microseconds {
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};
    use time::UtcDateTime;

    /// Serializes `stamp` as an integer of whole Unix microseconds, dropping any fraction
    ///
    /// The multiplication cannot overflow while the `large-dates` feature of the `time` crate is
    /// off, which keeps years within 9999
    ///
    /// # Errors
    ///
    /// Returns the serializer's error when the integer cannot be written
    pub fn serialize<S: Serializer>(stamp: &UtcDateTime, serializer: S) -> Result<S::Ok, S::Error> {
        serializer
            .serialize_i64(stamp.unix_timestamp() * 1_000_000 + i64::from(stamp.microsecond()))
    }

    /// Deserializes an integer of Unix microseconds into a `UtcDateTime`
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error when the value is not an `i64` or lies outside the
    /// range of `UtcDateTime`
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<UtcDateTime, D::Error> {
        let micros = i64::deserialize(deserializer)?;

        UtcDateTime::from_unix_timestamp_nanos(i128::from(micros) * 1_000).map_err(D::Error::custom)
    }
}
