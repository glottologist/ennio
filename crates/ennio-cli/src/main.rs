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

    /// Manage remote node daemons
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },
}

#[derive(Subcommand)]
enum NodeAction {
    /// Check node health status
    Status {
        /// Remote host (if omitted, checks all configured nodes)
        host: Option<String>,
    },

    /// List all configured node projects
    List,

    /// Establish node connection for a project
    Connect {
        /// Project ID
        project: String,
    },

    /// Disconnect and optionally shutdown a remote node
    Disconnect {
        /// Project ID
        project: String,
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

    let cfg = cli.config.as_deref();

    match cli.command {
        Commands::Init { path } => commands::init(&path).await,
        Commands::Start => commands::start(cfg).await,
        Commands::Stop => commands::stop(cfg).await,
        Commands::Status { project } => {
            commands::status(project.as_deref(), cfg, &cli.format).await
        }
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
                cfg,
                &cli.format,
            )
            .await
        }
        Commands::Session { action } => match action {
            SessionAction::Info { id } => commands::session_info(&id, cfg, &cli.format).await,
            SessionAction::Kill { id } => commands::session_kill(&id, cfg).await,
            SessionAction::Restore { id } => commands::session_restore(&id, cfg, &cli.format).await,
            SessionAction::List { project } => {
                commands::status(project.as_deref(), cfg, &cli.format).await
            }
        },
        Commands::Send { session, message } => commands::send(&session, &message, cfg).await,
        Commands::Dashboard { port } => commands::dashboard(port).await,
        Commands::Open { session } => commands::open(&session, cfg).await,
        Commands::Node { action } => match action {
            NodeAction::Status { host } => commands::node_status(host.as_deref()).await,
            NodeAction::List => commands::node_list().await,
            NodeAction::Connect { project } => commands::node_connect(&project).await,
            NodeAction::Disconnect { project } => commands::node_disconnect(&project).await,
        },
    }
}
