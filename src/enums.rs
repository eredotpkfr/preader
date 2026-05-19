use anyhow::{Context, Result};
use pyo3::prelude::*;
use serde::de::DeserializeOwned;

#[pyclass(eq, eq_int, from_py_object)]
#[derive(Clone, PartialEq)]
pub enum PReaderConfigFormat {
    #[pyo3(name = "JSON")]
    Json,
    #[pyo3(name = "TOML")]
    Toml,
    #[pyo3(name = "YAML")]
    Yaml,
}

impl PReaderConfigFormat {
    pub(crate) fn parse<T: DeserializeOwned>(&self, content: &str) -> Result<T> {
        match self {
            Self::Json => serde_json::from_str(content).context("invalid JSON"),
            Self::Toml => toml::from_str(content).context("invalid TOML"),
            Self::Yaml => serde_norway::from_str(content).context("invalid YAML"),
        }
    }
}
