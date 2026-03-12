use crate::parser::Table;

/// Convert a parsed Table to Markdown format.
pub fn format_markdown(table: &Table) -> String {
    let mut lines = Vec::new();

    let escaped_headers: Vec<String> = table.headers.iter().map(|h| escape_pipe(h)).collect();
    let header = format!("| {} |", escaped_headers.join(" | "));
    lines.push(header);

    let sep: Vec<String> = table.headers.iter().map(|_| "---".to_string()).collect();
    lines.push(format!("| {} |", sep.join(" | ")));

    for row in &table.rows {
        let escaped: Vec<String> = row.iter().map(|c| escape_pipe(c)).collect();
        lines.push(format!("| {} |", escaped.join(" | ")));
    }

    lines.join("\n")
}

/// Escape pipe characters in cell content to prevent breaking Markdown table structure.
fn escape_pipe(text: &str) -> String {
    text.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Table;

    #[test]
    fn test_markdown_format() {
        let table = Table {
            headers: vec![
                "コマンド".to_string(),
                "動作".to_string(),
                "備考".to_string(),
            ],
            rows: vec![
                vec![
                    "cla -r".to_string(),
                    "直接起動".to_string(),
                    "基本".to_string(),
                ],
                vec![
                    "cla -r -auto".to_string(),
                    "自動モード".to_string(),
                    "推奨".to_string(),
                ],
            ],
        };
        let output = format_markdown(&table);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "| コマンド | 動作 | 備考 |");
        assert_eq!(lines[1], "| --- | --- | --- |");
        assert_eq!(lines[2], "| cla -r | 直接起動 | 基本 |");
        assert_eq!(lines[3], "| cla -r -auto | 自動モード | 推奨 |");
    }

    #[test]
    fn test_empty_rows() {
        let table = Table {
            headers: vec!["A".to_string(), "B".to_string()],
            rows: vec![],
        };
        let output = format_markdown(&table);
        assert_eq!(output.lines().count(), 2);
    }
}
