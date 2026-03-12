/// Parsed table: header row + data rows, each row is a Vec of cell strings.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Check if a character is a Unicode box-drawing vertical separator.
fn is_vertical(c: char) -> bool {
    matches!(c, '│' | '┃' | '║')
}

/// Check if a line is a border/separator line (contains horizontal box-drawing chars).
fn is_separator_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    // A separator line is composed of box-drawing characters and whitespace
    trimmed.chars().all(|c| {
        matches!(
            c,
            '─' | '━'
                | '═'
                | '┌'
                | '┐'
                | '└'
                | '┘'
                | '├'
                | '┤'
                | '┬'
                | '┴'
                | '┼'
                | '┏'
                | '┓'
                | '┗'
                | '┛'
                | '┣'
                | '┫'
                | '┳'
                | '┻'
                | '╋'
                | '╔'
                | '╗'
                | '╚'
                | '╝'
                | '╠'
                | '╣'
                | '╦'
                | '╩'
                | '╬'
                | ' '
        )
    })
}

/// Check if a line is a data row (contains vertical box-drawing separators).
fn is_data_line(line: &str) -> bool {
    line.chars().any(is_vertical)
}

/// Extract cell contents from a data line by splitting on vertical separators.
fn extract_cells(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();

    for c in line.chars() {
        if is_vertical(c) {
            cells.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(c);
        }
    }
    // Only add trailing content if there's non-whitespace after the last separator
    let trailing = current.trim().to_string();
    if !trailing.is_empty() {
        cells.push(trailing);
    }

    // Remove the first empty cell caused by leading border
    // e.g., "│ a │ b │" splits to ["", "a", "b"]
    // The trailing empty cell is already handled by not adding empty trailing content
    if cells.first().is_some_and(|s| s.is_empty()) {
        cells.remove(0);
    }

    cells
}

/// Parse input text and extract all Unicode box-drawing tables.
/// Returns None if no tables are found.
pub fn parse_tables(input: &str) -> Option<Vec<Table>> {
    let lines: Vec<&str> = input.lines().collect();
    let mut tables = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        // Look for the start of a table (separator line or data line)
        if is_separator_line(lines[i]) || is_data_line(lines[i]) {
            let mut data_rows: Vec<Vec<String>> = Vec::new();

            // Consume all lines that belong to this table
            while i < lines.len() && (is_separator_line(lines[i]) || is_data_line(lines[i])) {
                if is_data_line(lines[i]) {
                    let cells = extract_cells(lines[i]);
                    if !cells.is_empty() {
                        data_rows.push(cells);
                    }
                }
                i += 1;
            }

            // Need at least a header row
            if !data_rows.is_empty() {
                let headers = data_rows.remove(0);
                tables.push(Table {
                    headers,
                    rows: data_rows,
                });
            }
        } else {
            i += 1;
        }
    }

    if tables.is_empty() {
        None
    } else {
        Some(tables)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_box_table() {
        let input = "\
┌──────────┬────────────┐
│ コマンド │ 動作       │
├──────────┼────────────┤
│ cla -r   │ 直接起動   │
│ cla -auto│ 自動モード │
└──────────┴────────────┘";

        let tables = parse_tables(input).unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].headers, vec!["コマンド", "動作"]);
        assert_eq!(tables[0].rows.len(), 2);
        assert_eq!(tables[0].rows[0], vec!["cla -r", "直接起動"]);
        assert_eq!(tables[0].rows[1], vec!["cla -auto", "自動モード"]);
    }

    #[test]
    fn test_heavy_box_table() {
        let input = "\
┏━━━━━┳━━━━━┓
┃ A   ┃ B   ┃
┣━━━━━╋━━━━━┫
┃ 1   ┃ 2   ┃
┗━━━━━┻━━━━━┛";

        let tables = parse_tables(input).unwrap();
        assert_eq!(tables[0].headers, vec!["A", "B"]);
        assert_eq!(tables[0].rows[0], vec!["1", "2"]);
    }

    #[test]
    fn test_double_box_table() {
        let input = "\
╔═════╦═════╗
║ X   ║ Y   ║
╠═════╬═════╣
║ 10  ║ 20  ║
╚═════╩═════╝";

        let tables = parse_tables(input).unwrap();
        assert_eq!(tables[0].headers, vec!["X", "Y"]);
        assert_eq!(tables[0].rows[0], vec!["10", "20"]);
    }

    #[test]
    fn test_no_table() {
        let input = "This is just plain text\nwith no table at all.";
        assert!(parse_tables(input).is_none());
    }

    #[test]
    fn test_empty_cells() {
        let input = "\
┌─────┬─────┐
│ A   │     │
├─────┼─────┤
│     │ B   │
└─────┴─────┘";

        let tables = parse_tables(input).unwrap();
        assert_eq!(tables[0].headers, vec!["A", ""]);
        assert_eq!(tables[0].rows[0], vec!["", "B"]);
    }

    #[test]
    fn test_single_column() {
        let input = "\
┌───────┐
│ Items │
├───────┤
│ one   │
│ two   │
└───────┘";

        let tables = parse_tables(input).unwrap();
        assert_eq!(tables[0].headers, vec!["Items"]);
        assert_eq!(tables[0].rows.len(), 2);
    }

    #[test]
    fn test_header_only() {
        let input = "\
┌─────┬─────┐
│ A   │ B   │
└─────┴─────┘";

        let tables = parse_tables(input).unwrap();
        assert_eq!(tables[0].headers, vec!["A", "B"]);
        assert_eq!(tables[0].rows.len(), 0);
    }

    #[test]
    fn test_japanese_content() {
        let input = "\
┌────────────────┬──────────────────────────┐
│ 機能           │ 説明                     │
├────────────────┼──────────────────────────┤
│ テーブル変換   │ Unicode罫線→Markdown     │
│ クリップボード │ 自動読み書き             │
└────────────────┴──────────────────────────┘";

        let tables = parse_tables(input).unwrap();
        assert_eq!(tables[0].headers, vec!["機能", "説明"]);
        assert_eq!(tables[0].rows.len(), 2);
        assert_eq!(tables[0].rows[0][0], "テーブル変換");
    }
}
