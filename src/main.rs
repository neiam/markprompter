use comrak::{markdown_to_html, ComrakOptions};
use eframe::{egui, epaint::Color32, App, CreationContext};
use egui::ScrollArea;
use egui_material_icons::icons::*;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Theme {
    name: String,
    background_color: [u8; 3],
    text_color: [u8; 3],
    heading_colors: Vec<[u8; 3]>,
}

// Application state
struct MarkPrompter {
    // File management
    current_file: Option<PathBuf>,
    content: String,
    parsed_content: String,

    // Scroll control
    scroll_position: f32,
    scroll_speed: f32, // pixels per second
    is_playing: bool,
    last_update: Instant,

    // Display settings
    font_size: f32,

    // Feature toggles
    pause_at_headings: bool,
    auto_restart: bool,
    heading_pause_duration: f32,
    current_heading_pause: Option<f32>,
    heading_line_indices: Vec<usize>,
    last_checked_heading_idx: usize,

    // Theme
    current_theme: Theme,
    available_themes: Vec<Theme>,

    // File watcher
    _file_watcher_tx: Option<Sender<()>>,
    file_watcher_rx: Option<Receiver<()>>,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            name: "Default".to_string(),
            background_color: [40, 44, 52],
            text_color: [220, 223, 228],
            heading_colors: vec![
                [255, 180, 100], // H1
                [230, 160, 90],  // H2
                [210, 140, 80],  // H3
                [190, 120, 70],  // H4
                [170, 100, 60],  // H5
                [150, 80, 50],   // H6
            ],
        }
    }
}

impl Default for MarkPrompter {
    fn default() -> Self {
        MarkPrompter {
            current_file: None,
            content: String::new(),
            parsed_content: String::new(),
            scroll_position: 0.0,
            scroll_speed: 50.0,
            is_playing: false,
            last_update: Instant::now(),
            font_size: 18.0,
            pause_at_headings: false,
            auto_restart: false,
            heading_pause_duration: 2.0,
            current_heading_pause: None,
            heading_line_indices: Vec::new(),
            last_checked_heading_idx: 0,
            current_theme: Theme::default(),
            available_themes: vec![Theme::default()],
            _file_watcher_tx: None,
            file_watcher_rx: None,
        }
    }
}

impl MarkPrompter {
    fn new(cc: &CreationContext) -> Self {
        // Initialize material icons
        egui_material_icons::initialize(&cc.egui_ctx);

        // Configure fonts
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::proportional(32.0)),
            (egui::TextStyle::Body, egui::FontId::proportional(18.0)),
            (egui::TextStyle::Monospace, egui::FontId::monospace(14.0)),
            (egui::TextStyle::Button, egui::FontId::proportional(16.0)),
            (egui::TextStyle::Small, egui::FontId::proportional(10.0)),
        ]
        .into();
        cc.egui_ctx.set_style(style);

        // Load themes from config file if it exists
        let mut app = Self::default();
        match load_themes_and_preference() {
            Ok((themes, saved_theme)) => {
                println!("Themes loaded successfully: {} themes", themes.len());
                app.available_themes = themes;

                // Load saved theme preference
                if let Some(saved_theme_name) = saved_theme {
                    // Try to find and set the saved theme
                    if let Some(saved_theme) = app
                        .available_themes
                        .iter()
                        .find(|t| t.name == saved_theme_name.trim())
                        .cloned()
                    {
                        println!("Restored saved theme: {}", saved_theme.name);
                        app.current_theme = saved_theme;
                    } else if !app.available_themes.is_empty() {
                        // Fallback to first theme if saved theme not found
                        app.current_theme = app.available_themes[0].clone();
                    }
                } else if !app.available_themes.is_empty() {
                    // No saved preference, use first theme
                    app.current_theme = app.available_themes[0].clone();
                }
            }
            Err(e) => {
                println!("Error loading themes: {}", e);
            }
        }

        app
    }

    // Parse and render inline markdown formatting
    fn render_formatted_text(
        &self,
        ui: &mut egui::Ui,
        text: &str,
        base_color: Color32,
        base_size: f32,
    ) {
        use egui::{text::LayoutJob, FontId, TextFormat};

        let mut job = LayoutJob::default();
        let mut chars = text.chars().peekable();
        let mut current_text = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '*' | '_' => {
                    // Check for bold or italic
                    if let Some(&next_ch) = chars.peek() {
                        if next_ch == ch {
                            // Double marker - bold
                            chars.next(); // consume second marker

                            // Add any pending text
                            if !current_text.is_empty() {
                                job.append(
                                    &current_text,
                                    0.0,
                                    TextFormat {
                                        font_id: FontId::proportional(base_size),
                                        color: base_color,
                                        ..Default::default()
                                    },
                                );
                                current_text.clear();
                            }

                            // Find closing markers
                            let mut content = String::new();
                            let mut found_closing = false;

                            while let Some(inner_ch) = chars.next() {
                                if inner_ch == ch {
                                    if let Some(&next_inner) = chars.peek() {
                                        if next_inner == ch {
                                            chars.next(); // consume second closing marker
                                            found_closing = true;
                                            break;
                                        }
                                    }
                                }
                                content.push(inner_ch);
                            }

                            if found_closing {
                                // Bold text - use larger size to simulate bold
                                job.append(
                                    &content,
                                    0.0,
                                    TextFormat {
                                        font_id: FontId::proportional(base_size * 1.15),
                                        color: base_color,
                                        ..Default::default()
                                    },
                                );
                            } else {
                                // No closing found, treat as normal text
                                current_text.push(ch);
                                current_text.push(ch);
                                current_text.push_str(&content);
                            }
                        } else {
                            // Single marker - italic
                            // Add any pending text
                            if !current_text.is_empty() {
                                job.append(
                                    &current_text,
                                    0.0,
                                    TextFormat {
                                        font_id: FontId::proportional(base_size),
                                        color: base_color,
                                        ..Default::default()
                                    },
                                );
                                current_text.clear();
                            }

                            // Find closing marker
                            let mut content = String::new();
                            let mut found_closing = false;

                            while let Some(inner_ch) = chars.next() {
                                if inner_ch == ch {
                                    found_closing = true;
                                    break;
                                }
                                content.push(inner_ch);
                            }

                            if found_closing {
                                // Italic text - use slightly smaller and different color
                                job.append(
                                    &content,
                                    0.0,
                                    TextFormat {
                                        font_id: FontId::proportional(base_size * 0.95),
                                        color: Color32::from_rgb(
                                            (base_color.r() as f32 * 0.9) as u8,
                                            (base_color.g() as f32 * 0.9) as u8,
                                            (base_color.b() as f32 * 0.9) as u8,
                                        ),
                                        italics: true,
                                        ..Default::default()
                                    },
                                );
                            } else {
                                // No closing found, treat as normal text
                                current_text.push(ch);
                                current_text.push_str(&content);
                            }
                        }
                    } else {
                        current_text.push(ch);
                    }
                }
                '`' => {
                    // Code formatting
                    if !current_text.is_empty() {
                        job.append(
                            &current_text,
                            0.0,
                            TextFormat {
                                font_id: FontId::proportional(base_size),
                                color: base_color,
                                ..Default::default()
                            },
                        );
                        current_text.clear();
                    }

                    let mut content = String::new();
                    let mut found_closing = false;

                    while let Some(inner_ch) = chars.next() {
                        if inner_ch == '`' {
                            found_closing = true;
                            break;
                        }
                        content.push(inner_ch);
                    }

                    if found_closing {
                        // Code text with background
                        job.append(
                            &content,
                            0.0,
                            TextFormat {
                                font_id: FontId::monospace(base_size * 0.9),
                                color: base_color,
                                background: Color32::from_rgba_premultiplied(80, 80, 80, 40),
                                ..Default::default()
                            },
                        );
                    } else {
                        current_text.push('`');
                        current_text.push_str(&content);
                    }
                }
                _ => {
                    current_text.push(ch);
                }
            }
        }

        // Add any remaining text
        if !current_text.is_empty() {
            job.append(
                &current_text,
                0.0,
                TextFormat {
                    font_id: FontId::proportional(base_size),
                    color: base_color,
                    ..Default::default()
                },
            );
        }

        ui.label(job);
    }

    fn open_file(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Markdown", &["md", "markdown"])
            .pick_file()
        {
            self.load_file(path);
        }
    }

    fn load_file(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.content = content;
                self.parse_markdown();
                self.current_file = Some(path.clone());
                self.scroll_position = 0.0;
                self.last_checked_heading_idx = 0;

                // Set up file watcher
                let (tx, rx) = channel();
                self.file_watcher_rx = Some(rx);

                let path_clone = path.clone();
                let watcher_tx = tx.clone();
                thread::spawn(move || {
                    let mut last_modified = fs::metadata(&path_clone)
                        .ok()
                        .map(|m| m.modified().ok())
                        .flatten();

                    loop {
                        thread::sleep(Duration::from_secs(1));

                        if let Ok(metadata) = fs::metadata(&path_clone) {
                            if let Ok(modified) = metadata.modified() {
                                if let Some(last) = last_modified {
                                    if modified > last {
                                        let _ = watcher_tx.send(());
                                        last_modified = Some(modified);
                                    }
                                } else {
                                    last_modified = Some(modified);
                                }
                            }
                        }
                    }
                });

                self._file_watcher_tx = Some(tx);
            }
            Err(e) => {
                eprintln!("Error loading file: {}", e);
            }
        }
    }

    fn parse_markdown(&mut self) {
        let mut options = ComrakOptions::default();
        options.extension.strikethrough = true;
        options.extension.table = true;
        options.extension.tasklist = true;
        options.extension.footnotes = true;

        self.parsed_content = markdown_to_html(&self.content, &options);

        // Extract heading positions for pause-at-headings feature if enabled
        if self.pause_at_headings {
            self.extract_heading_positions();
        }
    }

    fn extract_heading_positions(&mut self) {
        // Simple approach: check each line for markdown heading markers
        let lines: Vec<&str> = self.content.lines().collect();
        let mut heading_line_indices = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("# ")
                || line.starts_with("## ")
                || line.starts_with("### ")
                || line.starts_with("#### ")
                || line.starts_with("##### ")
                || line.starts_with("###### ")
            {
                heading_line_indices.push(i);
            }
        }

        // We'll use this information in the update_scroll method
        self.heading_line_indices = heading_line_indices;
    }

    fn check_file_updates(&mut self) {
        if let Some(rx) = &self.file_watcher_rx {
            if rx.try_recv().is_ok() {
                if let Some(path) = &self.current_file {
                    if let Ok(content) = fs::read_to_string(path) {
                        self.content = content;
                        self.parse_markdown();
                    }
                }
            }
        }
    }

    fn update_scroll(&mut self, dt: f32) {
        if !self.is_playing {
            return;
        }

        // Handle heading pause if enabled
        if let Some(remaining) = self.current_heading_pause {
            if remaining > 0.0 {
                self.current_heading_pause = Some(remaining - dt);
                return;
            } else {
                self.current_heading_pause = None;
            }
        }

        // Calculate new scroll position
        self.scroll_position += self.scroll_speed * dt;

        // Check if we should pause at a heading
        if self.pause_at_headings && !self.heading_line_indices.is_empty() {
            // Calculate approximate line based on scroll position and font size
            let approximate_line = (self.scroll_position / (self.font_size * 1.5)) as usize;

            // Check if we're approaching a heading
            for (idx, &heading_line) in self
                .heading_line_indices
                .iter()
                .enumerate()
                .skip(self.last_checked_heading_idx)
            {
                // If we've scrolled past this heading
                if approximate_line >= heading_line && idx >= self.last_checked_heading_idx {
                    // Pause scrolling for the specified duration
                    self.current_heading_pause = Some(self.heading_pause_duration);
                    self.last_checked_heading_idx = idx + 1;
                    break;
                }
            }
        }
    }
}

impl App for MarkPrompter {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_file_updates();

        // Set background color from theme
        let bg_color = Color32::from_rgb(
            self.current_theme.background_color[0],
            self.current_theme.background_color[1],
            self.current_theme.background_color[2],
        );

        let mut style = (*ctx.style()).clone();
        style.visuals.panel_fill = bg_color;
        style.visuals.window_fill = bg_color;
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Use columns with custom width ratio - give more space to controls panel
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(300.0, ui.available_height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        // Left column - Controls panel
                        ui.heading("MP");
                        ui.add_space(10.0);

                        // File controls
                        if ui
                            .add_sized(
                                [80.0, 80.0],
                                egui::Button::new(
                                    egui::RichText::new(format!("{} ", ICON_FOLDER_OPEN))
                                        .size(28.0),
                                ),
                            )
                            .clicked()
                        {
                            self.open_file();
                        }

                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Playback controls
                        ui.heading("Playback");
                        ui.add_space(5.0);

                        ui.horizontal(|ui| {
                            let play_pause_text = if self.is_playing {
                                egui::RichText::new(ICON_PAUSE).size(48.0)
                            } else {
                                egui::RichText::new(ICON_PLAY_ARROW).size(48.0)
                            };

                            if ui
                                .add_sized([80.0, 80.0], egui::Button::new(play_pause_text))
                                .clicked()
                            {
                                self.is_playing = !self.is_playing;
                                self.last_update = Instant::now();
                            }

                            if ui
                                .add_sized(
                                    [80.0, 80.0],
                                    egui::Button::new(
                                        egui::RichText::new(ICON_SKIP_PREVIOUS).size(48.0),
                                    ),
                                )
                                .clicked()
                            {
                                self.scroll_position = 0.0;
                                self.last_checked_heading_idx = 0;
                            }
                        });

                        ui.add_space(10.0);

                        // Speed controls
                        ui.label("Scroll Speed");
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    [60.0, 60.0],
                                    egui::Button::new(egui::RichText::new(ICON_REMOVE).size(36.0)),
                                )
                                .clicked()
                            {
                                self.scroll_speed = (self.scroll_speed - 10.0).max(10.0);
                            }
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new(format!("{}px/s", self.scroll_speed as i32))
                                    .size(20.0),
                            );
                            ui.add_space(10.0);
                            if ui
                                .add_sized(
                                    [60.0, 60.0],
                                    egui::Button::new(egui::RichText::new(ICON_ADD).size(36.0)),
                                )
                                .clicked()
                            {
                                self.scroll_speed = (self.scroll_speed + 10.0).min(500.0);
                            }
                        });

                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Feature toggles
                        ui.heading("Settings");
                        ui.add_space(5.0);

                        ui.checkbox(&mut self.pause_at_headings, "Pause at Headings");

                        if self.pause_at_headings {
                            ui.horizontal(|ui| {
                                ui.label("Duration:");
                                ui.add(
                                    egui::Slider::new(&mut self.heading_pause_duration, 0.5..=10.0)
                                        .suffix("s")
                                        .text("sec"),
                                );
                            });
                        }

                        ui.checkbox(&mut self.auto_restart, "Auto Restart");

                        ui.add_space(5.0);

                        // Font size
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    [50.0, 50.0],
                                    egui::Button::new(
                                        egui::RichText::new(ICON_TEXT_DECREASE).size(32.0),
                                    ),
                                )
                                .clicked()
                            {
                                self.font_size = (self.font_size - 1.0).max(8.0);
                            }
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new(format!("{:.0}px", self.font_size)).size(20.0),
                            );
                            ui.add_space(10.0);
                            if ui
                                .add_sized(
                                    [50.0, 50.0],
                                    egui::Button::new(
                                        egui::RichText::new(ICON_TEXT_INCREASE).size(32.0),
                                    ),
                                )
                                .clicked()
                            {
                                self.font_size = (self.font_size + 1.0).min(72.0);
                            }
                        });

                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Theme selection
                        ui.heading("Theme");
                        ui.add_space(5.0);

                        egui::ComboBox::from_label("Select Theme")
                            .selected_text(self.current_theme.name.clone())
                            .show_ui(ui, |ui| {
                                for theme in &self.available_themes {
                                    if ui
                                        .selectable_label(
                                            self.current_theme.name == theme.name,
                                            theme.name.clone(),
                                        )
                                        .clicked()
                                    {
                                        self.current_theme = theme.clone();
                                        // Save theme preference
                                        if let Err(e) = save_theme_preference(&theme.name) {
                                            eprintln!("Failed to save theme preference: {}", e);
                                        }
                                    }
                                }
                            });
                    },
                );

                ui.separator();

                // Right column - Content panel
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.available_height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        if let Some(file) = &self.current_file {
                            ui.heading(
                                file.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                            );
                        }

                        let text_color = Color32::from_rgb(
                            self.current_theme.text_color[0],
                            self.current_theme.text_color[1],
                            self.current_theme.text_color[2],
                        );

                        // Fill remaining height with scroll area
                        let available_size = ui.available_size();
                        let scroll_area = ScrollArea::vertical()
                            .max_height(available_size.y)
                            .max_width(available_size.x)
                            .vertical_scroll_offset(self.scroll_position);

                        let output = scroll_area.show(ui, |ui| {
                            ui.set_width(available_size.x - 20.0); // Account for scrollbar

                            // Calculate time delta for scrolling
                            let now = Instant::now();
                            let dt = now.duration_since(self.last_update).as_secs_f32();
                            self.last_update = now;
                            self.update_scroll(dt);

                            if !self.parsed_content.is_empty() {
                                // Custom markdown rendering with colored headings
                                let lines = self.content.lines().collect::<Vec<&str>>();

                                egui::Grid::new("markdown_content")
                                    .num_columns(1)
                                    .spacing([0.0, 5.0])
                                    .striped(false)
                                    .show(ui, |ui| {
                                        for (_i, line) in lines.iter().enumerate() {
                                            let trimmed = line.trim();

                                            // Detect heading level and extract text without #
                                            let mut heading_level = 0;
                                            let display_text = if trimmed.starts_with("# ") {
                                                heading_level = 1;
                                                trimmed.trim_start_matches("# ")
                                            } else if trimmed.starts_with("## ") {
                                                heading_level = 2;
                                                trimmed.trim_start_matches("## ")
                                            } else if trimmed.starts_with("### ") {
                                                heading_level = 3;
                                                trimmed.trim_start_matches("### ")
                                            } else if trimmed.starts_with("#### ") {
                                                heading_level = 4;
                                                trimmed.trim_start_matches("#### ")
                                            } else if trimmed.starts_with("##### ") {
                                                heading_level = 5;
                                                trimmed.trim_start_matches("##### ")
                                            } else if trimmed.starts_with("###### ") {
                                                heading_level = 6;
                                                trimmed.trim_start_matches("###### ")
                                            } else {
                                                *line
                                            };

                                            // Apply appropriate color and styling based on whether it's a heading
                                            if heading_level > 0
                                                && heading_level
                                                    <= self.current_theme.heading_colors.len()
                                            {
                                                // It's a heading - use the appropriate heading color
                                                let idx = heading_level - 1;
                                                let heading_color = Color32::from_rgb(
                                                    self.current_theme.heading_colors[idx][0],
                                                    self.current_theme.heading_colors[idx][1],
                                                    self.current_theme.heading_colors[idx][2],
                                                );

                                                // Adjust font size based on heading level
                                                // H1: 2.0x, H2: 1.8x, H3: 1.6x, H4: 1.4x, H5: 1.2x, H6: 1.1x
                                                // let size_multipliers = [2.0, 1.8, 1.6, 1.4, 1.2, 1.1];
                                                let size_multipliers =
                                                    [2.0, 1.8, 1.6, 1.4, 1.2, 1.1];
                                                let heading_size =
                                                    self.font_size * size_multipliers[idx];
                                                ui.style_mut()
                                                    .text_styles
                                                    .get_mut(&egui::TextStyle::Body)
                                                    .unwrap()
                                                    .size = heading_size;

                                                ui.colored_label(heading_color, display_text);
                                                ui.end_row();

                                                // Reset font size to default
                                                ui.style_mut()
                                                    .text_styles
                                                    .get_mut(&egui::TextStyle::Body)
                                                    .unwrap()
                                                    .size = self.font_size;
                                            } else {
                                                // Regular text - use the formatted text renderer
                                                self.render_formatted_text(
                                                    ui,
                                                    display_text,
                                                    text_color,
                                                    self.font_size,
                                                );
                                                ui.end_row();
                                            }
                                        }
                                    });
                            } else {
                                ui.colored_label(text_color, "Open a markdown file to begin.");
                            }
                        });

                        // Handle end-of-content scrolling behavior
                        if self.is_playing {
                            let content_height = output.inner_rect.height();
                            let available_height = ui.available_height();

                            if self.scroll_position >= content_height - available_height {
                                if self.auto_restart {
                                    self.scroll_position = 0.0;
                                    self.last_checked_heading_idx = 0;
                                } else {
                                    self.scroll_position =
                                        (content_height - available_height).max(0.0);
                                    self.is_playing = false;
                                }
                            }
                        }
                    },
                );
            });
        });

        // Request continuous repaint to enable smooth scrolling
        ctx.request_repaint();
    }
}

// Save theme preference to themes.toml
fn save_theme_preference(theme_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = "themes.toml";

    // Read the current themes
    let themes = load_themes_without_preference()?;

    // Create the config structure with preference
    #[derive(Serialize)]
    struct ThemesConfigWithPreference {
        selected_theme: String,
        themes: Vec<Theme>,
    }

    let config = ThemesConfigWithPreference {
        selected_theme: theme_name.to_string(),
        themes,
    };

    let toml_string = toml::to_string(&config)?;
    fs::write(config_path, toml_string)?;
    Ok(())
}

// Load themes and preference from a TOML file
fn load_themes_and_preference() -> Result<(Vec<Theme>, Option<String>), Box<dyn std::error::Error>>
{
    let config_path = "themes.toml";
    if !std::path::Path::new(config_path).exists() {
        // Create a default theme file if it doesn't exist
        let default_themes = create_default_themes();

        println!("Attempting to create themes.toml file...");

        // Wrap themes in a structure for TOML serialization
        #[derive(Serialize)]
        struct ThemesConfig {
            selected_theme: Option<String>,
            themes: Vec<Theme>,
        }

        let config = ThemesConfig {
            selected_theme: None,
            themes: default_themes.clone(),
        };

        let toml_string = toml::to_string(&config)?;
        println!("TOML string generated successfully");
        fs::write(config_path, toml_string)?;
        println!("themes.toml file created successfully");
        return Ok((default_themes, None));
    }

    let toml_str = fs::read_to_string(config_path)?;

    // Parse TOML with optional selected_theme field
    #[derive(Deserialize)]
    struct ThemesWrapperWithPreference {
        selected_theme: Option<String>,
        themes: Vec<Theme>,
    }

    // Try parsing with selected_theme field
    match toml::from_str::<ThemesWrapperWithPreference>(&toml_str) {
        Ok(wrapper) => Ok((wrapper.themes, wrapper.selected_theme)),
        Err(_) => {
            // Fallback: try parsing without selected_theme (old format)
            #[derive(Deserialize)]
            struct ThemesWrapper {
                themes: Vec<Theme>,
            }

            let wrapper: ThemesWrapper = toml::from_str(&toml_str)?;
            Ok((wrapper.themes, None))
        }
    }
}

// Load themes without preference (for saving)
fn load_themes_without_preference() -> Result<Vec<Theme>, Box<dyn std::error::Error>> {
    let (themes, _) = load_themes_and_preference()?;
    Ok(themes)
}

// Helper function to create default themes
fn create_default_themes() -> Vec<Theme> {
    vec![
        Theme {
            name: "Light".to_string(),
            background_color: [240, 240, 245],
            text_color: [60, 60, 70],
            heading_colors: vec![
                [100, 100, 180], // H1
                [90, 90, 170],   // H2
                [80, 80, 160],   // H3
                [70, 70, 150],   // H4
                [60, 60, 140],   // H5
                [50, 50, 130],   // H6
            ],
        },
        Theme {
            name: "Dark".to_string(),
            background_color: [40, 44, 52],
            text_color: [220, 223, 228],
            heading_colors: vec![
                [255, 180, 100], // H1
                [230, 160, 90],  // H2
                [210, 140, 80],  // H3
                [190, 120, 70],  // H4
                [170, 100, 60],  // H5
                [150, 80, 50],   // H6
            ],
        },
        Theme {
            name: "Solarized".to_string(),
            background_color: [0, 43, 54],
            text_color: [131, 148, 150],
            heading_colors: vec![
                [181, 137, 0],   // H1
                [203, 75, 22],   // H2
                [220, 50, 47],   // H3
                [211, 54, 130],  // H4
                [108, 113, 196], // H5
                [38, 139, 210],  // H6
            ],
        },
        Theme {
            name: "After Dark".to_string(),
            background_color: [32, 29, 101], // base-100: #201D65
            text_color: [172, 171, 213],     // secondary: #ACABD5
            heading_colors: vec![
                [254, 243, 199], // accent: #fef3c7 - H1
                [123, 121, 181], // primary: #7B79B5 - H2
                [172, 171, 213], // secondary: #ACABD5 - H3
                [125, 211, 252], // info: #7dd3fc - H4
                [167, 243, 208], // success: #a7f3d0 - H5
                [254, 240, 138], // warning: #fef08a - H6
            ],
        },
        Theme {
            name: "Her".to_string(),
            background_color: [101, 29, 29], // base-100: #651d1d
            text_color: [213, 171, 171],     // secondary: #d5abab
            heading_colors: vec![
                [254, 243, 199], // accent: #fef3c7 - H1
                [181, 121, 121], // primary: #b57979 - H2
                [213, 171, 171], // secondary: #d5abab - H3
                [125, 211, 252], // info: #7dd3fc - H4
                [167, 243, 208], // success: #a7f3d0 - H5
                [254, 240, 138], // warning: #fef08a - H6
            ],
        },
        Theme {
            name: "Forest".to_string(),
            background_color: [5, 46, 22], // base-100: #052e16
            text_color: [134, 239, 172],   // secondary: #86efac
            heading_colors: vec![
                [254, 243, 199], // accent: #fef3c7 - H1
                [74, 222, 128],  // primary: #4ade80 - H2
                [134, 239, 172], // secondary: #86efac - H3
                [125, 211, 252], // info: #7dd3fc - H4
                [167, 243, 208], // success: #a7f3d0 - H5
                [254, 240, 138], // warning: #fef08a - H6
            ],
        },
        Theme {
            name: "Sky".to_string(),
            background_color: [8, 47, 73], // base-100: #082f49
            text_color: [125, 211, 252],   // secondary: #7dd3fc
            heading_colors: vec![
                [254, 243, 199], // accent: #fef3c7 - H1
                [56, 189, 248],  // primary: #38bdf8 - H2
                [125, 211, 252], // secondary: #7dd3fc - H3
                [167, 243, 208], // success: #a7f3d0 - H4
                [254, 240, 138], // warning: #fef08a - H5
                [252, 165, 165], // error: #fca5a5 - H6
            ],
        },
        Theme {
            name: "Clays".to_string(),
            background_color: [69, 26, 3], // base-100: #451a03
            text_color: [245, 158, 11],    // secondary: #f59e0b
            heading_colors: vec![
                [254, 243, 199], // accent: #fef3c7 - H1
                [217, 119, 6],   // primary: #d97706 - H2
                [245, 158, 11],  // secondary: #f59e0b - H3
                [125, 211, 252], // info: #7dd3fc - H4
                [167, 243, 208], // success: #a7f3d0 - H5
                [254, 240, 138], // warning: #fef08a - H6
            ],
        },
        Theme {
            name: "Stones".to_string(),
            background_color: [41, 37, 36], // base-100: #292524
            text_color: [156, 163, 175],    // secondary: #9ca3af
            heading_colors: vec![
                [254, 243, 199], // accent: #fef3c7 - H1
                [107, 114, 128], // primary: #6b7280 - H2
                [156, 163, 175], // secondary: #9ca3af - H3
                [125, 211, 252], // info: #7dd3fc - H4
                [167, 243, 208], // success: #a7f3d0 - H5
                [254, 240, 138], // warning: #fef08a - H6
            ],
        },
    ]
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "MarkPrompter",
        options,
        Box::new(|cc| Ok(Box::new(MarkPrompter::new(cc)))),
    )
}
