use async_trait::async_trait;

use crate::error::EnnioError;
use crate::session::Session;

#[async_trait]
pub trait Terminal: Send + Sync {
    fn name(&self) -> &str;

    async fn open_session(&self, session: &Session) -> Result<(), EnnioError>;

    async fn open_all(&self, sessions: &[Session]) -> Result<(), EnnioError>;

    async fn is_session_open(&self, session: &Session) -> Result<bool, EnnioError>;
}
