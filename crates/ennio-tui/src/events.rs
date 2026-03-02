use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};

use crate::error::TuiError;

#[derive(Debug)]
pub enum TerminalEvent {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
}

pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    pub fn next(&self) -> Result<TerminalEvent, TuiError> {
        if event::poll(self.tick_rate).map_err(|e| TuiError::EventPoll {
            message: e.to_string(),
        })? {
            let raw = event::read().map_err(|e| TuiError::EventPoll {
                message: e.to_string(),
            })?;
            match raw {
                Event::Key(key) => Ok(TerminalEvent::Key(key)),
                Event::Resize(w, h) => Ok(TerminalEvent::Resize(w, h)),
                _ => Ok(TerminalEvent::Tick),
            }
        } else {
            Ok(TerminalEvent::Tick)
        }
    }
}
