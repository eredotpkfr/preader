pub fn indent_lines(text: &str, width: usize) -> String {
    text.replace('\n', &format!("\n{}", " ".repeat(width)))
}
