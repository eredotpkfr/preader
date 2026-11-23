pub mod seconds {
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};
    use time::UtcDateTime;

    pub fn serialize<S: Serializer>(stamp: &UtcDateTime, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(stamp.unix_timestamp())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<UtcDateTime, D::Error> {
        UtcDateTime::from_unix_timestamp(i64::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

pub mod microseconds {
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};
    use time::UtcDateTime;

    pub fn serialize<S: Serializer>(stamp: &UtcDateTime, serializer: S) -> Result<S::Ok, S::Error> {
        serializer
            .serialize_i64(stamp.unix_timestamp() * 1_000_000 + i64::from(stamp.microsecond()))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<UtcDateTime, D::Error> {
        let micros = i64::deserialize(deserializer)?;

        UtcDateTime::from_unix_timestamp_nanos(i128::from(micros) * 1_000).map_err(D::Error::custom)
    }
}
