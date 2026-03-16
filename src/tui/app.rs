//! TUI application state.

use crate::scanner::{get_scanner, PortEntry, ScanOptions};
use crate::filter::{filter_entries, FilterOptions};
use crate::sort::{sort_entries, SortField, SortOptions};
use super::ui;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::time::Duration;

/// Application state.
pub struct App {
    /// Should quit.
    pub should_quit: bool,
    /// Port entries.
    pub entries: Vec<PortEntry>,
    /// Selected index.
    pub selected: usize,
    /// Scan options.
    pub scan_opts: ScanOptions,
    /// Filter options.
    pub filter_opts: FilterOptions,
    /// Sort options.
    pub sort_opts: SortOptions,
    /// Filter input mode.
    pub filter_mode: bool,
    /// Filter input buffer.
    pub filter_input: String,
    /// Show help.
    pub show_help: bool,
    /// Show detail popup.
    pub show_detail: bool,
    /// Show kill confirmation.
    pub confirm_kill: bool,
    /// Status message (for feedback).
    pub status_message: Option<String>,
}

impl App {
    pub fn new(scan_opts: ScanOptions) -> Self {
        Self {
            should_quit: false,
            entries: Vec::new(),
            selected: 0,
            scan_opts,
            filter_opts: FilterOptions::default(),
            sort_opts: SortOptions::default(),
            filter_mode: false,
            filter_input: String::new(),
            show_help: false,
            show_detail: false,
            confirm_kill: false,
            status_message: None,
        }
    }
    
    /// Refresh the port list.
    pub fn refresh(&mut self) {
        let scanner = get_scanner();
        if let Ok(entries) = scanner.list_ports(&self.scan_opts) {
            let entries = filter_entries(entries, &self.filter_opts);
            let entries = sort_entries(entries, &self.sort_opts);
            self.entries = entries;
            
            // Clamp selection
            if self.selected >= self.entries.len() && !self.entries.is_empty() {
                self.selected = self.entries.len() - 1;
            }
        }
    }
    
    /// Move selection down.
    pub fn next(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1) % self.entries.len();
        }
    }
    
    /// Move selection up.
    pub fn previous(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.entries.len() - 1);
        }
    }
    
    /// Go to top.
    pub fn go_top(&mut self) {
        self.selected = 0;
    }
    
    /// Go to bottom.
    pub fn go_bottom(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
    }
    
    /// Get selected entry.
    pub fn selected_entry(&self) -> Option<&PortEntry> {
        self.entries.get(self.selected)
    }
    
    /// Toggle external filter.
    pub fn toggle_external(&mut self) {
        self.filter_opts.external_only = !self.filter_opts.external_only;
        self.filter_opts.localhost_only = false;
        self.refresh();
    }
    
    /// Toggle localhost filter.
    pub fn toggle_localhost(&mut self) {
        self.filter_opts.localhost_only = !self.filter_opts.localhost_only;
        self.filter_opts.external_only = false;
        self.refresh();
    }
    
    /// Cycle sort field.
    pub fn cycle_sort(&mut self) {
        self.sort_opts.field = match self.sort_opts.field {
            SortField::Port => SortField::Process,
            SortField::Process => SortField::Memory,
            SortField::Memory => SortField::Cpu,
            SortField::Cpu => SortField::Time,
            SortField::Time => SortField::Port,
        };
        self.refresh();
    }
    
    /// Run the TUI.
    pub fn run(&mut self) -> io::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        
        // Initial refresh
        self.refresh();
        
        // Main loop
        let result = self.run_loop(&mut terminal);
        
        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        
        result
    }
    
    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
        let tick_rate = Duration::from_millis(2000);
        let mut last_tick = std::time::Instant::now();
        
        loop {
            // Draw UI
            terminal.draw(|f| ui::draw(f, self))?;
            
            // Handle events
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let event::Event::Key(key) = event::read()? {
                    if self.filter_mode {
                        match key.code {
                            KeyCode::Esc => {
                                self.filter_mode = false;
                                self.filter_input.clear();
                                self.filter_opts.process = None;
                                self.refresh();
                            }
                            KeyCode::Enter => {
                                self.filter_mode = false;
                                self.filter_opts.process = if self.filter_input.is_empty() {
                                    None
                                } else {
                                    Some(self.filter_input.clone())
                                };
                                self.refresh();
                            }
                            KeyCode::Backspace => {
                                self.filter_input.pop();
                            }
                            KeyCode::Char(c) => {
                                self.filter_input.push(c);
                            }
                            _ => {}
                        }
                    } else if self.show_help {
                        self.show_help = false;
                    } else if self.show_detail {
                        // Any key closes detail view
                        self.show_detail = false;
                    } else if self.confirm_kill {
                        // Kill confirmation dialog
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                // Confirmed - kill with SIGTERM then SIGKILL
                                if let Some(entry) = self.selected_entry().cloned() {
                                    if let Some(pid) = entry.pid {
                                        match crate::process::kill_process(
                                            pid,
                                            &crate::process::KillOptions::new(),
                                        ) {
                                            Ok(()) => {
                                                self.status_message = Some(format!(
                                                    "Killed {} (PID {})",
                                                    entry.process_name.as_deref().unwrap_or("process"),
                                                    pid
                                                ));
                                            }
                                            Err(e) => {
                                                self.status_message = Some(format!("Kill failed: {}", e));
                                            }
                                        }
                                        self.refresh();
                                    }
                                }
                                self.confirm_kill = false;
                            }
                            _ => {
                                // Any other key cancels
                                self.confirm_kill = false;
                                self.status_message = Some("Kill cancelled".to_string());
                            }
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                self.should_quit = true;
                            }
                            KeyCode::Char('j') | KeyCode::Down => self.next(),
                            KeyCode::Char('k') | KeyCode::Up => self.previous(),
                            KeyCode::Char('g') => self.go_top(),
                            KeyCode::Char('G') => self.go_bottom(),
                            KeyCode::Char('r') | KeyCode::Char('R') => self.refresh(),
                            KeyCode::Char('/') => {
                                self.filter_mode = true;
                                self.filter_input.clear();
                            }
                            KeyCode::Char('e') => self.toggle_external(),
                            KeyCode::Char('l') => self.toggle_localhost(),
                            KeyCode::Char('s') => self.cycle_sort(),
                            KeyCode::Char('?') | KeyCode::Char('h') => {
                                self.show_help = true;
                            }
                            KeyCode::Enter => {
                                // Show detail view for selected port
                                if self.selected_entry().is_some() {
                                    self.show_detail = true;
                                }
                            }
                            KeyCode::Char('K') => {
                                // Show kill confirmation dialog
                                if self.selected_entry().is_some() {
                                    self.confirm_kill = true;
                                    self.status_message = None;
                                }
                            }
                            KeyCode::Char('S') => {
                                // Send SIGTERM only (no SIGKILL fallback)
                                if let Some(entry) = self.selected_entry().cloned() {
                                    if let Some(pid) = entry.pid {
                                        match crate::process::send_sigterm(pid) {
                                            Ok(()) => {
                                                self.status_message = Some(format!(
                                                    "Sent SIGTERM to {} (PID {})",
                                                    entry.process_name.as_deref().unwrap_or("process"),
                                                    pid
                                                ));
                                            }
                                            Err(e) => {
                                                self.status_message = Some(format!("SIGTERM failed: {}", e));
                                            }
                                        }
                                        self.refresh();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            // Check for quit
            if self.should_quit {
                return Ok(());
            }
            
            // Auto-refresh
            if last_tick.elapsed() >= tick_rate {
                self.refresh();
                last_tick = std::time::Instant::now();
            }
        }
    }
}
