# MarkPrompter

A professional Rust-based markdown teleprompter with automatic scrolling, live preview, and rich text formatting support. Built with egui for smooth, cross-platform performance.

## Features

### Core Functionality
- **Markdown File Viewing**: Load and display markdown files with rich formatting
- **Automatic Scrolling**: Smooth, continuous scrolling at adjustable speeds
- **Live File Reloading**: Automatically updates content when the file changes
- **Rich Text Formatting**: Full support for markdown inline formatting

### Markdown Support
- **Headings (H1-H6)**: Displayed without `#` symbols with progressive sizing
  - H1: 2.0x base font size
  - H2: 1.8x base font size
  - H3: 1.6x base font size
  - H4: 1.4x base font size
  - H5: 1.2x base font size
  - H6: 1.1x base font size
- **Bold Text**: `**text**` or `__text__` - rendered with larger font size
- **Italic Text**: `*text*` or `_text_` - rendered with subtle styling
- **Inline Code**: `` `code` `` - monospace font with background highlight

### Playback Controls
- **Play/Pause**: Start and stop automatic scrolling with visual button feedback
- **Restart**: Jump back to the beginning of the document instantly
- **Speed Control**: Fine-tune scrolling speed from 10-500 pixels per second
- **Smooth Scrolling**: Frame-rate independent smooth motion

### Advanced Features
- **Pause at Headings**: Automatically pause scrolling when reaching markdown headings
  - Configurable pause duration (0.5-10 seconds)
  - Smart detection of all heading levels
- **Auto-Restart**: Loop content continuously for unattended presentations
- **Adjustable Font Size**: Scale text from 8-72px for optimal readability

### Theme System
- **9 Built-in Themes**: 
  - Light - Clean and bright for well-lit environments
  - Dark - Easy on the eyes for extended use
  - Solarized - A Popular color scheme
  - After Dark - Purple-based theme 
  - Her - Warm red tones
  - Forest - Natural green palette
  - Sky - Cool blue theme
  - Clays - Earthy brown tones
  - Stones - Neutral gray theme
- **Theme Persistence**: Your selected theme is automatically saved and restored
- **TOML Configuration**: Easy theme customization via `themes.toml`
- **Per-Heading Colors**: Each heading level can have its own color

### User Interface
- **Clean Layout**: Intuitive control panel with clear visual hierarchy
- **Material Icons**: Professional iconography throughout the interface
- **Responsive Design**: Minimum window size of 800x600, scales to any resolution
- **Real-time Preview**: See changes instantly as you adjust settings

## Usage

1. **Launch the application**: Run `cargo run` or the compiled executable
2. **Load a markdown file**: Click the folder icon to select your `.md` file
3. **Start presenting**: Click the play button to begin auto-scrolling
4. **Customize as needed**: 
   - Adjust scroll speed with +/- buttons
   - Change font size for your audience
   - Select a theme that matches your environment
   - Enable heading pauses for emphasis

## Keyboard Shortcuts

- **Space**: Play/Pause (coming soon)
- **R**: Restart from beginning (coming soon)
- **+/-**: Adjust font size (coming soon)

## Building

### Prerequisites
- MSRV: Rust 1.86

### Required Packages

## RPM
gtk3-devel gdk-pixbuf-devel pango-devel

## DEB
gtk3-dev gdk-pixbuf-dev pango-dev

### Build Commands

```bash
# Development build
cargo build

# Optimized release build
cargo run --release

# Run directly
cargo run
```

## Configuration

### Theme Configuration

Themes are stored in `themes.toml` with the following structure:

```toml
# Selected theme is saved here
selected_theme = "Dark"

[[themes]]
name = "My Custom Theme"
background_color = [40, 44, 52]      # RGB values
text_color = [220, 223, 228]         # RGB values
heading_colors = [
    [255, 180, 100],  # H1 color
    [230, 160, 90],   # H2 color
    [210, 140, 80],   # H3 color
    [190, 120, 70],   # H4 color
    [170, 100, 60],   # H5 color
    [150, 80, 50],    # H6 color
]
```