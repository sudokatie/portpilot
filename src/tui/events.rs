//! TUI event handling.

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Application events.
#[allow(dead_code)]
#[derive(Debug)]
pub enum Event {
    /// Key press.
    Key(KeyEvent),
    /// Terminal tick.
    Tick,
}

/// Event handler.
#[allow(dead_code)]
pub struct Events {
    rx: mpsc::Receiver<Event>,
    _tx: mpsc::Sender<Event>,
}

#[allow(dead_code)]
impl Events {
    /// Create a new event handler with tick rate.
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let event_tx = tx.clone();
        
        thread::spawn(move || {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    if let Ok(CrosstermEvent::Key(key)) = event::read() {
                        if event_tx.send(Event::Key(key)).is_err() {
                            return;
                        }
                    }
                }
                if event_tx.send(Event::Tick).is_err() {
                    return;
                }
            }
        });
        
        Self { rx, _tx: tx }
    }
    
    /// Get next event.
    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
