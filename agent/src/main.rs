use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, EnvFilter};

mod commands;

#[derive(Parser)]
#[command(
    name = "linklink",
    about = "Secure mesh tunnel — connect devices privately across any network",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Log in to the control plane (device authorization flow)
    Login {
        /// Control plane URL
        #[arg(long)]
        server: Option<String>,
    },

    /// Register this device with the control plane
    Register {
        /// Human-readable device name
        #[arg(long)]
        name: String,
    },

    /// Create a serverless network invitation (no server needed)
    Invite(InviteArgs),

    /// Start the WireGuard tunnel
    Up,

    /// Stop the WireGuard tunnel
    Down,

    /// Show tunnel and peer status
    Status,

    /// List known peers
    Peers,

    /// Manage relay mode for this device
    Relay(RelayArgs),

    /// Remove all local config and keys (irreversible)
    Reset,
}

#[derive(Parser)]
struct InviteArgs {
    #[command(subcommand)]
    action: InviteAction,
}

#[derive(Subcommand)]
enum InviteAction {
    /// Generate a new invitation code (and QR)
    Create {
        /// Device name to include in the invitation
        #[arg(long, default_value = "my-device")]
        name: String,
    },
    /// Accept an invitation from another device
    Accept {
        /// The invitation code (LINKLINK:v1:...)
        code: String,
    },
}

#[derive(Parser)]
struct RelayArgs {
    #[command(subcommand)]
    action: RelayAction,
}

#[derive(Subcommand)]
enum RelayAction {
    /// Enable relay mode (requires public IP)
    Enable,
    /// Disable relay mode
    Disable,
    /// Initialize hub keypair (first-time setup on relay VPS)
    Init,
    /// Register this relay with the control plane
    Register {
        #[arg(long)]
        server: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let log_level = if cli.verbose { "debug" } else { "info" };
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .with_target(false)
        .init();

    let result = match cli.command {
        Commands::Login { server } => commands::auth::cmd_login(server).await,
        Commands::Register { name } => commands::register::cmd_register(name).await,
        Commands::Invite(a) => match a.action {
            InviteAction::Create { name } => commands::invite::cmd_invite_create(name).await,
            InviteAction::Accept { code } => commands::invite::cmd_invite_accept(code).await,
        },
        Commands::Up => commands::tunnel::cmd_up().await,
        Commands::Down => commands::tunnel::cmd_down().await,
        Commands::Status => commands::status::cmd_status().await,
        Commands::Peers => commands::peers::cmd_peers().await,
        Commands::Relay(a) => match a.action {
            RelayAction::Enable => commands::relay::cmd_relay_enable().await,
            RelayAction::Disable => commands::relay::cmd_relay_disable().await,
            RelayAction::Init => commands::relay::cmd_relay_init().await,
            RelayAction::Register { server } => {
                commands::relay::cmd_relay_register(server).await
            }
        },
        Commands::Reset => commands::tunnel::cmd_reset().await,
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
