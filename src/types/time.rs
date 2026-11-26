use serde::{Deserialize, Serialize};
use time::UtcDateTime;

use crate::utils;

/// Creation and last update times of a state, in whole UTC seconds
///
/// `updated_at` changes only when the state is saved or resynced, not while an iterator advances
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "preader", eq, frozen, get_all)
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Timestamps {
    /// When the state was first created
    #[serde(with = "crate::serializers::time::seconds")]
    pub created_at: UtcDateTime,
    /// When the state was last saved or resynced
    #[serde(with = "crate::serializers::time::seconds")]
    pub updated_at: UtcDateTime,
}

impl Timestamps {
    /// Returns timestamps that are both the current UTC time, truncated to the second
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::Timestamps;
    ///
    /// let timestamps = Timestamps::now();
    ///
    /// assert_eq!(timestamps.created_at, timestamps.updated_at);
    /// ```
    pub fn now() -> Self {
        let now = utils::time::now();

        Self {
            created_at: now,
            updated_at: now,
        }
    }
}
