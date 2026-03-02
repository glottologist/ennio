use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod format;

#[derive(Parser)]
#[command(
    name = "ennio",
    version,
    about = "Agent orchestrator for parallel AI coding agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(long, default_value = "table")]
    format: format::OutputFormat,

    /// Config file path
    #[arg(long, short)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new ennio config
    Init {
        /// Project path
        #[arg(default_value = ".")]
        path: String,
    },

    /// Start the orchestrator lifecycle loop
    Start,

    /// Stop the orchestrator lifecycle loop
    Stop,

    /// Show status of all sessions
    Status {
        /// Filter by project
        project: Option<String>,
    },

    /// Spawn a new agent session
    Spawn {
        /// Project ID
        project: String,

        /// Issue ID to work on
        #[arg(short, long)]
        issue: Option<String>,

        /// Direct prompt
        #[arg(short, long)]
        prompt: Option<String>,

        /// Branch name
        #[arg(short, long)]
        branch: Option<String>,

        /// Session role
        #[arg(short, long)]
        role: Option<String>,
    },

    /// Manage a specific session
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },

    /// Send a message to a running session
    Send {
        /// Session ID
        session: String,

        /// Message to send
        message: String,
    },

    /// Open the web dashboard
    Dashboard {
        /// Port to run on
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },

    /// Open session terminal
    Open {
        /// Session ID (or "all" for all sessions)
        session: String,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// Show session details
    Info {
        /// Session ID
        id: String,
    },

    /// Kill a session
    Kill {
        /// Session ID
        id: String,
    },

    /// Restore an exited session
    Restore {
        /// Session ID
        id: String,
    },

    /// List all sessions
    List {
        /// Filter by project
        project: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    ennio_observe::logging::init_logging();

    match cli.command {
        Commands::Init { path } => commands::init(&path).await,
        Commands::Start => commands::start(cli.config.as_deref()).await,
        Commands::Stop => commands::stop().await,
        Commands::Status { project } => commands::status(project.as_deref(), &cli.format).await,
        Commands::Spawn {
            project,
            issue,
            prompt,
            branch,
            role,
        } => {
            commands::spawn(
                &project,
                issue.as_deref(),
                prompt.as_deref(),
                branch.as_deref(),
                role.as_deref(),
                &cli.format,
            )
            .await
        }
        Commands::Session { action } => match action {
            SessionAction::Info { id } => commands::session_info(&id, &cli.format).await,
            SessionAction::Kill { id } => commands::session_kill(&id).await,
            SessionAction::Restore { id } => commands::session_restore(&id, &cli.format).await,
            SessionAction::List { project } => {
                commands::status(project.as_deref(), &cli.format).await
            }
        },
        Commands::Send { session, message } => commands::send(&session, &message).await,
        Commands::Dashboard { port } => commands::dashboard(port).await,
        Commands::Open { session } => commands::open(&session).await,
    }
}
