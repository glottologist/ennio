mod desktop;
mod slack;
mod webhook;

pub use desktop::DesktopNotifier;
pub use slack::SlackNotifier;
pub use webhook::WebhookNotifier;
