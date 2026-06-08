use anyhow::Result;
use clap::{Parser, ValueEnum};
use reticulumchat::{identity::Identity, AppRuntime, AppState, Config};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "reticulumchat")]
#[command(about = "A chat client for the Reticulum mesh network")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Run mode: TUI or GUI
    #[arg(short, long, value_enum, default_value = "tui")]
    mode: RunMode,

    /// Reticulum host address
    #[arg(short, long, default_value = "127.0.0.1")]
    host: String,

    /// Reticulum port
    #[arg(short, long, default_value_t = 3742)]
    port: u16,

    /// Path to identity file
    #[arg(short, long, default_value = "~/.reticulumchat/identity")]
    identity: String,

    /// Disable desktop notifications
    #[arg(long)]
    no_notifications: bool,

    /// Disable end-to-end encryption
    #[arg(long)]
    no_encryption: bool,

    /// Display name for this node
    #[arg(short, long, default_value = "anonymous")]
    name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum RunMode {
    /// Terminal User Interface mode
    Tui,
    /// Graphical User Interface mode (requires gui feature)
    Gui,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Load or generate identity
    let identity = match load_identity(&cli.identity).await {
        Ok(id) => {
            tracing::info!("Loaded identity: {}", id.name);
            id
        }
        Err(e) => {
            tracing::warn!("Failed to load identity ({}), generating new one", e);
            let id = Identity::generate(&cli.name)?;
            if let Err(e) = save_identity(&cli.identity, &id).await {
                tracing::warn!("Failed to save identity: {}", e);
            }
            id
        }
    };

    // Build configuration
    let config = Config {
        identity_path: cli.identity,
        reticulum_host: cli.host,
        reticulum_port: cli.port,
        enable_notifications: !cli.no_notifications,
        enable_encryption: !cli.no_encryption,
    };

    // Create shared application state
    let state = Arc::new(RwLock::new(AppState::new(config, identity)));
    let _runtime = AppRuntime::new(state.clone());

    // Launch selected mode
    match cli.mode {
        RunMode::Tui => {
            tracing::info!("Starting TUI mode...");
            let mut app = reticulumchat::tui::TuiApp::new(state);
            app.run().await?;
        }
        RunMode::Gui => {
            #[cfg(feature = "gui")]
            {
                tracing::info!("Starting GUI mode...");
                let app = reticulumchat::gui::GuiApp::new(state);
                app.run()?;
            }
            #[cfg(not(feature = "gui"))]
            {
                anyhow::bail!(
                    "GUI mode requested but the 'gui' feature is not enabled. \
                     Rebuild with: cargo build --features gui"
                );
            }
        }
    }

    Ok(())
}

async fn load_identity(path: &str) -> Result<Identity> {
    let expanded = shellexpand::tilde(path);
    let data = tokio::fs::read_to_string(expanded.as_ref()).await?;
    let identity: Identity = serde_json::from_str(&data)?;
    Ok(identity)
}

async fn save_identity(path: &str, identity: &Identity) -> Result<()> {
    let expanded = shellexpand::tilde(path);
    let parent = std::path::Path::new(expanded.as_ref())
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid identity path"))?;
    tokio::fs::create_dir_all(parent).await?;
    let data = serde_json::to_string_pretty(identity)?;
    tokio::fs::write(expanded.as_ref(), data).await?;
    Ok(())
}
