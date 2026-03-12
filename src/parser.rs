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

/// Check if a character is CJK or fullwidth (no space needed when joining wrapped text).
fn is_cjk_or_fullwidth(c: char) -> bool {
    let cp = c as u32;
    matches!(
        cp,
        0x2E80..=0x9FFF
            | 0xAC00..=0xD7AF
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF01..=0xFFEF
            | 0x20000..=0x2FA1F
    )
}

/// Merge multiple physical lines into one logical row.
/// Used when a table cell wraps across multiple terminal lines.
fn merge_group(group: &[Vec<String>]) -> Vec<String> {
    if group.len() == 1 {
        return group[0].clone();
    }
    let max_cols = group.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut merged = Vec::with_capacity(max_cols);
    for col in 0..max_cols {
        let mut result = String::new();
        for row in group {
            let cell = row.get(col).map(|s| s.trim()).unwrap_or("");
            if cell.is_empty() {
                continue;
            }
            if result.is_empty() {
                result = cell.to_string();
            } else {
                let last_char = result.chars().last().unwrap();
                let first_char = cell.chars().next().unwrap();
                if is_cjk_or_fullwidth(last_char) || is_cjk_or_fullwidth(first_char) {
                    result.push_str(cell);
                } else {
                    result.push(' ');
                    result.push_str(cell);
                }
            }
        }
        merged.push(result);
    }
    merged
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
            // Group data lines between separator lines
            let mut groups: Vec<Vec<Vec<String>>> = Vec::new();
            let mut current_group: Vec<Vec<String>> = Vec::new();

            while i < lines.len() && (is_separator_line(lines[i]) || is_data_line(lines[i])) {
                if is_separator_line(lines[i]) {
                    if !current_group.is_empty() {
                        groups.push(current_group);
                        current_group = Vec::new();
                    }
                } else {
                    let cells = extract_cells(lines[i]);
                    if !cells.is_empty() {
                        current_group.push(cells);
                    }
                }
                i += 1;
            }
            if !current_group.is_empty() {
                groups.push(current_group);
            }

            if groups.is_empty() {
                continue;
            }

            let headers;
            let rows;

            if groups.len() == 1 {
                // No separators between rows (or only top/bottom borders)
                let all_lines = &groups[0];
                if all_lines.is_empty() {
                    continue;
                }
                headers = all_lines[0].clone();
                rows = all_lines[1..].to_vec();
            } else if groups.len() == 2 {
                // Only header separator, no row separators in body
                headers = merge_group(&groups[0]);
                rows = groups[1].clone();
            } else {
                // 3+ groups → table has row separators → merge each group
                headers = merge_group(&groups[0]);
                rows = groups[1..].iter().map(|g| merge_group(g)).collect();
            }

            if !headers.is_empty() {
                tables.push(Table { headers, rows });
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
    fn test_multiline_cells() {
        let input = "\
┌──────────────────┬───────────────┬──────┬──────────────┐
│ 分類             │ 内容          │ 行   │ 判定         │
│                  │               │ 数   │              │
├──────────────────┼───────────────┼──────┼──────────────┤
│ 確定仕様         │ 概要、基本    │ ~50  │ 残す         │
│                  │ 仕様          │ 行   │              │
├──────────────────┼───────────────┼──────┼──────────────┤
│ 確定仕様（プロン │ preserve版    │ 12   │ 残す（コアロジ│
│ プト）           │               │ 行   │ ック）       │
└──────────────────┴───────────────┴──────┴──────────────┘";

        let tables = parse_tables(input).unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].headers, vec!["分類", "内容", "行数", "判定"]);
        assert_eq!(tables[0].rows.len(), 2);
        assert_eq!(
            tables[0].rows[0],
            vec!["確定仕様", "概要、基本仕様", "~50行", "残す"]
        );
        assert_eq!(
            tables[0].rows[1],
            vec!["確定仕様（プロンプト）", "preserve版", "12行", "残す（コアロジック）"]
        );
    }

    #[test]
    fn test_multiline_with_row_separators_preserves_simple_rows() {
        // Table with row separators but single-line cells should still work
        let input = "\
┌────────────┬──────┬──────────┐
│ 猫種       │ 毛種 │ 毛質     │
├────────────┼──────┼──────────┤
│ ミヌエット │ 長短 │ ふわふわ │
├────────────┼──────┼──────────┤
│ マンチカン │ 長短 │ やわらか │
├────────────┼──────┼──────────┤
│ ベンガル   │ 短毛 │ シルク   │
└────────────┴──────┴──────────┘";

        let tables = parse_tables(input).unwrap();
        assert_eq!(tables[0].headers, vec!["猫種", "毛種", "毛質"]);
        assert_eq!(tables[0].rows.len(), 3);
        assert_eq!(tables[0].rows[0], vec!["ミヌエット", "長短", "ふわふわ"]);
        assert_eq!(tables[0].rows[2], vec!["ベンガル", "短毛", "シルク"]);
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
