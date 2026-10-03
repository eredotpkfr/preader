use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "preader", eq, frozen, get_all, skip_from_py_object)
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Timestamps {
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
}

impl Timestamps {
    pub fn now() -> Self {
        let now = Utc::now();

        Self {
            created_at: now,
            updated_at: now,
        }
    }
}
