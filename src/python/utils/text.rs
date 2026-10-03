// Indentation applied to every nested representation, matching the Python side
const INDENT: &str = "\n  ";

pub(crate) fn nest(text: &str) -> String {
    text.replace('\n', INDENT)
}

pub(crate) fn quote(value: impl std::fmt::Display) -> String {
    format!("'{value}'")
}
