use serde::{Deserialize, Serialize};
use time::UtcDateTime;

use crate::utils;

#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "preader", eq, frozen, get_all)
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Timestamps {
    #[serde(with = "crate::serializers::time::seconds")]
    pub created_at: UtcDateTime,
    #[serde(with = "crate::serializers::time::seconds")]
    pub updated_at: UtcDateTime,
}

impl Timestamps {
    pub fn now() -> Self {
        let now = utils::time::now();

        Self {
            created_at: now,
            updated_at: now,
        }
    }
}
