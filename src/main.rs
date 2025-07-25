use eframe::{egui, epaint::Color32, App, CreationContext};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::fs;
use egui::ScrollArea;
use rfd::FileDialog;
use comrak::{markdown_to_html, ComrakOptions};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

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
    scroll_speed: f32,  // pixels per second
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
        // Configure fonts
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::proportional(32.0)),
            (egui::TextStyle::Body, egui::FontId::proportional(18.0)),
            (egui::TextStyle::Monospace, egui::FontId::monospace(14.0)),
            (egui::TextStyle::Button, egui::FontId::proportional(16.0)),
            (egui::TextStyle::Small, egui::FontId::proportional(10.0)),
        ].into();
        cc.egui_ctx.set_style(style);
        
        // Load themes from config file if it exists
        let mut app = Self::default();
        if let Ok(themes) = load_themes() {
            app.available_themes = themes;
            if !app.available_themes.is_empty() {
                app.current_theme = app.available_themes[0].clone();
            }
        }
        
        app
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
                    let mut last_modified = fs::metadata(&path_clone).ok().map(|m| m.modified().ok()).flatten();
                    
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
            },
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
            if line.starts_with("# ") || line.starts_with("## ") || 
               line.starts_with("### ") || line.starts_with("#### ") || 
               line.starts_with("##### ") || line.starts_with("###### ") {
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
            for (idx, &heading_line) in self.heading_line_indices.iter().enumerate().skip(self.last_checked_heading_idx) {
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
            self.current_theme.background_color[2]
        );
        
        let mut style = (*ctx.style()).clone();
        style.visuals.panel_fill = bg_color;
        style.visuals.window_fill = bg_color;
        ctx.set_style(style);
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.set_height(ui.available_height());
                
                // Controls panel
                ui.vertical(|ui| {
                    ui.set_width(200.0);
                    ui.set_height(ui.available_height());
                    ui.heading("MarkPrompter");
                    ui.add_space(10.0);
                    
                    // File controls
                    if ui.button("Open File").clicked() {
                        self.open_file();
                    }
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);
                    
                    // Playback controls
                    ui.heading("Playback");
                    ui.add_space(5.0);
                    
                    if ui.button(if self.is_playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                        self.is_playing = !self.is_playing;
                        self.last_update = Instant::now();
                    }
                    
                    ui.add_space(5.0);
                    
                    if ui.button("⏮ Restart").clicked() {
                        self.scroll_position = 0.0;
                    }
                    
                    ui.add_space(10.0);
                    
                    // Speed controls
                    ui.label("Scroll Speed");
                    ui.horizontal(|ui| {
                        if ui.small_button("-").clicked() {
                            self.scroll_speed = (self.scroll_speed - 10.0).max(10.0);
                        }
                        ui.label(format!("{}px/s", self.scroll_speed as i32));
                        if ui.small_button("+").clicked() {
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
                            ui.label("Pause Duration:");
                            ui.add(egui::Slider::new(&mut self.heading_pause_duration, 0.5..=10.0)
                                .suffix("s")
                                .text("seconds"));
                        });
                    }
                    
                    ui.checkbox(&mut self.auto_restart, "Auto Restart");
                    
                    ui.add_space(5.0);
                    
                    // Font size
                    ui.horizontal(|ui| {
                        ui.label("Font Size:");
                        if ui.small_button("-").clicked() {
                            self.font_size = (self.font_size - 1.0).max(8.0);
                        }
                        ui.label(format!("{:.0}px", self.font_size));
                        if ui.small_button("+").clicked() {
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
                                if ui.selectable_label(
                                    self.current_theme.name == theme.name,
                                    theme.name.clone()
                                ).clicked() {
                                    self.current_theme = theme.clone();
                                }
                            }
                        });
                });
                
                ui.separator();
                
                // Content panel
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(ui.available_height());
                    
                    if let Some(file) = &self.current_file {
                        ui.heading(file.file_name().unwrap_or_default().to_string_lossy().to_string());
                    }
                    
                    let text_color = Color32::from_rgb(
                        self.current_theme.text_color[0],
                        self.current_theme.text_color[1],
                        self.current_theme.text_color[2]
                    );
                    
                    let scroll_area = ScrollArea::vertical()
                        .max_height(ui.available_height())
                        .vertical_scroll_offset(self.scroll_position);
                    
                    let output = scroll_area.show(ui, |ui| {
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
                                        
                                        // Detect heading level
                                        let mut heading_level = 0;
                                        if trimmed.starts_with("# ") { heading_level = 1; }
                                        else if trimmed.starts_with("## ") { heading_level = 2; }
                                        else if trimmed.starts_with("### ") { heading_level = 3; }
                                        else if trimmed.starts_with("#### ") { heading_level = 4; }
                                        else if trimmed.starts_with("##### ") { heading_level = 5; }
                                        else if trimmed.starts_with("###### ") { heading_level = 6; }
                                        
                                        // Apply appropriate color and styling based on whether it's a heading
                                        if heading_level > 0 && heading_level <= self.current_theme.heading_colors.len() {
                                            // It's a heading - use the appropriate heading color
                                            let idx = heading_level - 1;
                                            let heading_color = Color32::from_rgb(
                                                self.current_theme.heading_colors[idx][0],
                                                self.current_theme.heading_colors[idx][1],
                                                self.current_theme.heading_colors[idx][2]
                                            );
                                            
                                            // Adjust font size based on heading level
                                            let heading_size = self.font_size * (1.5 - (heading_level as f32 * 0.1));
                                            ui.style_mut().text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = heading_size;
                                            
                                            ui.colored_label(heading_color, *line);
                                            ui.end_row();
                                            
                                            // Reset font size to default
                                            ui.style_mut().text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = self.font_size;
                                        } else {
                                            // Regular text - use the default text color
                                            ui.colored_label(text_color, *line);
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
                                self.scroll_position = (content_height - available_height).max(0.0);
                                self.is_playing = false;
                            }
                        }
                    }
                });
            });
        });
        
        // Request continuous repaint to enable smooth scrolling
        ctx.request_repaint();
    }
}

// Load themes from a TOML file
fn load_themes() -> Result<Vec<Theme>, Box<dyn std::error::Error>> {
    let config_path = "themes.toml";
    if !std::path::Path::new(config_path).exists() {
        // Create a default theme file if it doesn't exist
        let default_themes = vec![
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
        ];
        
        let toml_string = toml::to_string(&default_themes)?;
        fs::write(config_path, toml_string)?;
        return Ok(default_themes);
    }
    
    let toml_str = fs::read_to_string(config_path)?;
    
    // Parse TOML
    #[derive(Deserialize)]
    struct ThemesWrapper {
        themes: Vec<Theme>,
    }
    
    // Try parsing as an array of themes first
    let themes: Result<Vec<Theme>, _> = toml::from_str(&toml_str);
    
    match themes {
        Ok(themes) => Ok(themes),
        Err(_) => {
            // Try parsing with the wrapper structure
            let wrapper: ThemesWrapper = toml::from_str(&toml_str)?;
            Ok(wrapper.themes)
        }
    }
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
        Box::new(|cc| Box::new(MarkPrompter::new(cc)))
    )
}
