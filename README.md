# MarkPrompter

A Rust-based markdown file viewer with automatic scrolling capabilities, built with egui.

## Features

### Core Functionality
- **Markdown File Viewing**: Load and display markdown files with proper formatting
- **Automatic Scrolling**: Continuously scroll through content at adjustable speeds
- **Live File Reloading**: Automatically updates content when the file changes

### Playback Controls
- **Play/Pause**: Start and stop the automatic scrolling
- **Restart**: Jump back to the beginning of the document
- **Speed Control**: Adjust scrolling speed from 10-500 pixels per second

### Advanced Features
- **Pause at Headings**: Automatically pause scrolling when reaching markdown headings
- **Auto-Restart**: Automatically restart from the beginning when reaching the end
- **Adjustable Font Size**: Change the display font size for better readability

### Theme Support
- **Multiple Themes**: Comes with Light, Dark, and Solarized themes
- **TOML Configuration**: Themes are loaded from a `themes.toml` file
- **Custom Colors**: Each theme supports custom background, text, and heading colors

## Usage

1. Click "Open File" to select a markdown file
2. Click "Play" to start automatic scrolling
3. Adjust settings as needed using the control panel

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

## Theme Configuration

Themes are stored in `themes.toml` and follow this format:

```toml
[[themes]]
name = "My Theme"
background_color = [40, 44, 52]
text_color = [220, 223, 228]
heading_colors = [
    [255, 180, 100], # H1
    [230, 160, 90],  # H2
    [210, 140, 80],  # H3
    [190, 120, 70],  # H4
    [170, 100, 60],  # H5
    [150, 80, 50],   # H6
]
```
