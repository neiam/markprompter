# MarkPrompter Demo

## Introduction

This is a sample markdown file to demonstrate the **MarkPrompter** application. The application allows automatic scrolling through markdown content with customizable speeds and themes.

## Features

### Auto-Scrolling

The auto-scrolling feature lets you read through documents hands-free. You can:

- Adjust the scrolling speed
- Pause and resume scrolling
- Restart from the beginning

### Theme Customization

MarkPrompter comes with several built-in themes:

1. Light
2. Dark
3. Solarized
4. Forest
5. Sepia

Each theme customizes:
- Background color
- Text color
- Heading colors

### Additional Settings

You can configure MarkPrompter to:

- Pause at headings automatically
- Auto-restart when reaching the end
- Adjust font size for better readability

## How It Works

The application uses:

```rust
// Example Rust code
fn update_scroll(&mut self, ui: &Ui) {
    if !self.is_playing {
        return;
    }
    
    let now = Instant::now();
    let dt = now.duration_since(self.last_update).as_secs_f32();
    self.last_update = now;
    
    // Calculate new scroll position
    let new_position = self.scroll_position + self.scroll_speed * dt;
    
    // Update scroll position
    self.scroll_position = new_position;
}
```

## Summary

MarkPrompter is designed to help you read through documents at a comfortable pace without manual scrolling. It's perfect for:

- Reading articles
- Following tutorials
- Reviewing documentation
- Studying notes

---

### Thank you for trying MarkPrompter!

Feel free to contribute to the project or report issues on GitHub.
