use crate::format::format_markdown;
use crate::parser::parse_tables;

/// Convert input text containing Unicode box-drawing tables to Markdown.
/// Returns None if no tables were found in the input.
pub fn convert_to_markdown(input: &str) -> Option<String> {
    let tables = parse_tables(input)?;

    let result: Vec<String> = tables.iter().map(format_markdown).collect();

    Some(result.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple_table() {
        let input = "\
┌──────┬──────┐
│ Name │ Age  │
├──────┼──────┤
│ Alice│ 30   │
│ Bob  │ 25   │
└──────┴──────┘";

        let result = convert_to_markdown(input).unwrap();
        assert!(result.contains("| Name | Age |"));
        assert!(result.contains("| --- | --- |"));
        assert!(result.contains("| Alice | 30 |"));
        assert!(result.contains("| Bob | 25 |"));
    }

    #[test]
    fn test_convert_returns_none_for_plain_text() {
        let input = "Hello world\nNo tables here";
        assert!(convert_to_markdown(input).is_none());
    }
}
