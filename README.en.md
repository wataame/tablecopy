<p align="center">
  <img src="assets/logo.png" alt="tablecopy" width="100%">
</p>

<p align="center">
  <strong>A CLI tool that converts terminal tables to Markdown or images</strong>
</p>

<p align="center">
  <a href="README.md">日本語</a>
</p>

---

Unicode box-drawing tables from CLI tools like Claude Code break when pasted into Slack or Notion.

tablecopy converts them to Markdown or clean table images with a single keyboard shortcut.

## Usage

### 1. Copy a table from your terminal

```
┌────────────┬──────┬──────────┐
│ Cat Breed  │ Coat │ Texture  │
├────────────┼──────┼──────────┤
│ Minuet     │ Both │ Fluffy   │
├────────────┼──────┼──────────┤
│ Munchkin   │ Both │ Soft     │
├────────────┼──────┼──────────┤
│ Bengal     │ Short│ Silky    │
└────────────┴──────┴──────────┘
```

Pasting this directly into Slack or Notion breaks the formatting.

### 2. Press the shortcut to convert to Markdown

Press `Cmd+Ctrl+M` (macOS) to convert and copy Markdown to your clipboard.

```markdown
| Cat Breed | Coat | Texture |
| --- | --- | --- |
| Minuet | Both | Fluffy |
| Munchkin | Both | Soft |
| Bengal | Short | Silky |
```

Paste directly into Notion with `Cmd+V`.

### 3. Press again to convert to image

Press `Cmd+Ctrl+M` again within 30 seconds to convert to a table image.

<!-- <img src="assets/table-image-example.png" alt="Table image example" width="400"> -->

A clean table image with CJK and emoji support is copied to your clipboard. Paste directly into Slack.

### HUD Notification

A HUD notification appears on screen when conversion is complete.

<p align="center">
  <img src="assets/hud-example.png" alt="HUD notification" width="300">
</p>

| OS | Notification |
|---|---|
| macOS | Native HUD overlay |
| Windows | Toast notification |
| Linux | None (terminal output only) |

Toggle HUD display with the `--hud` option:

```bash
tablecopy --hud off   # Disable
tablecopy --hud on    # Enable (default)
```

## Installation

### Homebrew (macOS / Linux)

```bash
brew install wataame/tap/tablecopy
```

### Shell (macOS / Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/wataame/tablecopy/releases/latest/download/tablecopy-installer.sh | sh
```

### PowerShell (Windows)

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/wataame/tablecopy/releases/latest/download/tablecopy-installer.ps1 | iex"
```

### Cargo

```bash
cargo install tablecopy
```

### Set up the shortcut

After installing, register the keyboard shortcut:

```bash
tablecopy --install
```

| OS | Shortcut | Method |
|---|---|---|
| macOS | `Cmd+Ctrl+M` | Automator Quick Action |
| Windows | `Ctrl+Alt+M` | AutoHotkey v2 script |

> **Windows** requires [AutoHotkey v2](https://www.autohotkey.com/).

### Command line

You can also run tablecopy directly without the shortcut:

```bash
# Convert from clipboard (same as shortcut)
tablecopy

# Convert stdin to Markdown
echo "┌───┬───┐..." | tablecopy -
```

### Uninstall

```bash
tablecopy --uninstall
```

## Supported table formats

Automatically detects tables drawn with Unicode box-drawing characters.

| Type | Characters | Example |
|---|---|---|
| Light | `┌─┐│└┘├┤┬┴┼` | Claude Code default |
| Heavy | `┏━┓┃┗┛┣┫┳┻╋` | Some tools |
| Double | `╔═╗║╚╝╠╣╦╩╬` | Some tools |

Supports CJK characters and emoji in table content.

## Requirements

| Item | Requirement |
|---|---|
| OS | macOS, Windows 10+ |
| Build | Rust 1.70+ |
| Dependencies | None (standalone binary) |

## How it works

```
Copy a table from your terminal
  ↓
tablecopy reads the clipboard
  ↓
Detects and parses Unicode box-drawing tables
  ↓
1st press → Convert to Markdown, set clipboard
2nd press → Generate SVG → Render PNG at Retina 2x → Set as image clipboard
  ↓
Cycles between formats within 30 seconds
```

Image rendering uses [resvg](https://github.com/linebender/resvg), a pure Rust SVG renderer.

## Feedback

Found a bug or have a feature request? Feel free to open an [Issue](https://github.com/wataame/tablecopy/issues)!

PRs are welcome.

## License

[MIT](LICENSE)
