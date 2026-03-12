use crate::parser::Table;
use unicode_width::UnicodeWidthStr;

const SCALE: u32 = 2; // Retina 2x
const FONT_SIZE: f64 = 14.0;
const CELL_PAD_X: f64 = 16.0;
const CELL_PAD_Y: f64 = 10.0;
const CHAR_WIDTH_UNIT: f64 = 8.0; // px per unicode half-width unit
const MIN_COL_WIDTH: f64 = 60.0;

#[cfg(target_os = "macos")]
const TEXT_FONT: &str = "Hiragino Sans";
#[cfg(target_os = "macos")]
const EMOJI_FONT: &str = "Apple Color Emoji";

#[cfg(target_os = "windows")]
const TEXT_FONT: &str = "Yu Gothic UI";
#[cfg(target_os = "windows")]
const EMOJI_FONT: &str = "Segoe UI Emoji";

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const TEXT_FONT: &str = "Noto Sans CJK JP";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const EMOJI_FONT: &str = "Noto Color Emoji";

pub struct TableImage {
    pub width: usize,
    pub height: usize,
    pub rgba_data: Vec<u8>,
}

pub fn render_table(table: &Table) -> Option<TableImage> {
    let svg = generate_svg(table);
    render_svg(&svg)
}

/// Check if a character is an emoji (needs Apple Color Emoji font)
fn is_emoji(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        0x231A..=0x231B |     // Watch, Hourglass
        0x23E9..=0x23F3 |     // Media control
        0x23F8..=0x23FA |     // Media control
        0x25AA..=0x25AB |     // Squares
        0x25B6 | 0x25C0 |     // Play buttons
        0x25FB..=0x25FE |     // Squares
        0x2600..=0x27BF |     // Misc Symbols, Dingbats
        0x2934..=0x2935 |     // Arrows
        0x2B05..=0x2B07 |     // Arrows
        0x2B1B..=0x2B1C |     // Squares
        0x2B50 | 0x2B55 |     // Star, Circle
        0x3030 | 0x303D |     // Wavy dash, Part alternation mark
        0x3297 | 0x3299 |     // Circled Ideographs
        0xFE00..=0xFE0F |     // Variation selectors
        0x1F000..=0x1FAFF |   // Extended emoji block
        0x200D |              // ZWJ
        0x20E3 |              // Combining Enclosing Keycap
        0xE0020..=0xE007F     // Tags
    )
}

struct TextRun {
    text: String,
    is_emoji: bool,
    x_offset: f64,
}

/// Split text into runs of emoji and non-emoji characters.
/// Each run gets its own <text> element with the appropriate font.
fn split_text_runs(text: &str) -> Vec<TextRun> {
    let mut runs = Vec::new();
    let mut current = String::new();
    let mut in_emoji = false;
    let mut x = 0.0;

    for c in text.chars() {
        let emoji = is_emoji(c);
        if emoji != in_emoji && !current.is_empty() {
            let w = UnicodeWidthStr::width(current.as_str()) as f64 * CHAR_WIDTH_UNIT;
            runs.push(TextRun {
                text: current.clone(),
                is_emoji: in_emoji,
                x_offset: x - w,
            });
            current.clear();
        }
        in_emoji = emoji;
        current.push(c);
        x += UnicodeWidthStr::width(c.to_string().as_str()) as f64 * CHAR_WIDTH_UNIT;
    }

    if !current.is_empty() {
        let w = UnicodeWidthStr::width(current.as_str()) as f64 * CHAR_WIDTH_UNIT;
        runs.push(TextRun {
            text: current,
            is_emoji: in_emoji,
            x_offset: x - w,
        });
    }

    runs
}

/// Check if text contains any emoji characters
fn has_emoji(text: &str) -> bool {
    text.chars().any(is_emoji)
}

fn generate_svg(table: &Table) -> String {
    let num_cols = table.headers.len();
    let all_rows: Vec<&Vec<String>> = std::iter::once(&table.headers)
        .chain(table.rows.iter())
        .collect();
    let num_rows = all_rows.len();

    // Calculate column widths based on content
    let col_widths: Vec<f64> = (0..num_cols)
        .map(|col| {
            all_rows
                .iter()
                .map(|row| {
                    let text = row.get(col).map_or("", |s| s.as_str());
                    UnicodeWidthStr::width(text) as f64 * CHAR_WIDTH_UNIT + CELL_PAD_X * 2.0
                })
                .fold(0.0_f64, f64::max)
                .max(MIN_COL_WIDTH)
                .ceil()
        })
        .collect();

    let row_height = FONT_SIZE + CELL_PAD_Y * 2.0;
    let total_w: f64 = col_widths.iter().sum();
    let total_h = row_height * num_rows as f64;

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">",
        total_w, total_h
    ));

    // White background
    s.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"#FFFFFF\"/>",
        total_w, total_h
    ));

    // Header background
    s.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"#F5F5F5\"/>",
        total_w, row_height
    ));

    // Cell text
    for (ri, row) in all_rows.iter().enumerate() {
        let mut x = 0.0;
        let y = ri as f64 * row_height;
        for (ci, cw) in col_widths.iter().enumerate() {
            let text = row.get(ci).map_or("", |s| s.as_str());
            if !text.is_empty() {
                let tx = x + CELL_PAD_X;
                let ty = y + row_height / 2.0 + FONT_SIZE * 0.35;
                let fw = if ri == 0 { "600" } else { "400" };

                if has_emoji(text) {
                    // Split into separate <text> elements for emoji/non-emoji
                    for run in split_text_runs(text) {
                        let font = if run.is_emoji { EMOJI_FONT } else { TEXT_FONT };
                        let rx = tx + run.x_offset;
                        let escaped = escape_svg(&run.text);
                        s.push_str(&format!(
                            "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" fill=\"#1A1A1A\">{}</text>",
                            rx, ty, font, FONT_SIZE, fw, escaped
                        ));
                    }
                } else {
                    // No emoji — single <text> element
                    let escaped = escape_svg(text);
                    s.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" fill=\"#1A1A1A\">{}</text>",
                        tx, ty, TEXT_FONT, FONT_SIZE, fw, escaped
                    ));
                }
            }
            x += cw;
        }
    }

    // Grid lines (crispEdges for sharp 1px rendering)
    s.push_str("<g shape-rendering=\"crispEdges\">");

    // Horizontal lines
    for i in 0..=num_rows {
        let y = i as f64 * row_height;
        s.push_str(&format!(
            "<line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#E0E0E0\" stroke-width=\"1\"/>",
            y, total_w, y
        ));
    }

    // Vertical lines
    let mut x = 0.0;
    s.push_str(&format!(
        "<line x1=\"0\" y1=\"0\" x2=\"0\" y2=\"{}\" stroke=\"#E0E0E0\" stroke-width=\"1\"/>",
        total_h
    ));
    for cw in &col_widths {
        x += cw;
        s.push_str(&format!(
            "<line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"{}\" stroke=\"#E0E0E0\" stroke-width=\"1\"/>",
            x, x, total_h
        ));
    }

    s.push_str("</g>");

    s.push_str("</svg>");
    s
}

fn escape_svg(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_svg(svg_str: &str) -> Option<TableImage> {
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = resvg::usvg::Tree::from_str(svg_str, &opt).ok()?;
    let size = tree.size().to_int_size();
    let w = size.width() * SCALE;
    let h = size.height() * SCALE;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    let transform = resvg::tiny_skia::Transform::from_scale(SCALE as f32, SCALE as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Some(TableImage {
        width: w as usize,
        height: h as usize,
        rgba_data: pixmap.take(),
    })
}
