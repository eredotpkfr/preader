use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[pyclass(module = "preader", skip_from_py_object)]
#[derive(Clone, Deserialize, Serialize)]
pub struct Timestamps {
    #[pyo3(get)]
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[pyo3(get)]
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

#[pymethods]
impl Timestamps {
    pub fn __repr__(&self) -> String {
        crate::macros::pyrepr!("Timestamps" {
            created_at = self.created_at.timestamp(),
            updated_at = self.updated_at.timestamp(),
        })
    }
}
