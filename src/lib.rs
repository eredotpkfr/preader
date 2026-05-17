use pyo3::prelude::*;

/// Formats the sum of two numbers as string.
/// Adds two numbers together.
///
/// # Examples
///
/// ```
/// use preader::sum_as_string;
/// assert_eq!(sum_as_string(3, 2).unwrap(), 5);
/// ```
#[pyfunction]
pub fn sum_as_string(a: usize, b: usize) -> PyResult<usize> {
    Ok(a + b)
}

/// A Python module implemented in Rust.
#[pymodule]
mod preader {
    #[pymodule_export]
    use super::sum_as_string;
}
