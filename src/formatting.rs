pub fn trim_indent(lines: &[&str]) -> String {
    let base_indent = get_base_indent(lines);
    let prefix = " ".repeat(base_indent);

    let trimmed_lines: Vec<&str> = lines
        .iter()
        .map(move |line| line.strip_prefix(&prefix).unwrap_or(line))
        .collect();

    trimmed_lines.join("\n")
}

fn get_base_indent(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter(|p| !p.is_empty())
        .map(|l| get_indent(l))
        .min()
        .expect("Failed to get base indent")
}

fn get_indent(line: &str) -> usize {
    let mut i = 0;
    for c in line.chars() {
        if c.is_whitespace() {
            i += 1;
        } else {
            return i;
        }
    }

    0
}
