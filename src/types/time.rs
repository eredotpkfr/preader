use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[pyclass(from_py_object)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Timestamps {
    #[pyo3(get)]
    pub created_at: i64,
    #[pyo3(get)]
    pub updated_at: i64,
}

impl Timestamps {
    pub fn now() -> Self {
        let now = chrono::Utc::now().timestamp();

        Self {
            created_at: now,
            updated_at: now,
        }
    }
}
