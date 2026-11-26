#![forbid(unsafe_code)]

fn main() {
    #[cfg(feature = "python")]
    pyo3_build_config::add_libpython_rpath_link_args();
}
