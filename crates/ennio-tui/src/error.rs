use std::io;

#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("terminal I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("event polling error: {message}")]
    EventPoll { message: String },
}
