use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use reticulumchat::{crash_reporter, identity::Identity, migrate_config, AppRuntime, AppState, Config};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "reticulumchat")]
#[command(about = "A chat client for the Reticulum mesh network")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

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

#[derive(Debug, Subcommand)]
enum Commands {
    /// Migrate configuration file to the current format version
    ConfigMigrate {
        /// Path to the configuration file
        #[arg(short, long, default_value = "~/.reticulumchat/config.json")]
        config: String,

        /// Create a backup of the original file before migrating
        #[arg(short, long, default_value_t = true)]
        backup: bool,
    },
    /// Submit beta testing feedback
    Feedback {
        /// Feedback message or description
        #[arg(short, long)]
        message: Option<String>,

        /// Feedback category
        #[arg(short, long, value_enum, default_value = "general")]
        category: FeedbackCategory,

        /// Attach a file to the feedback (e.g., logs, screenshots)
        #[arg(short, long)]
        attachment: Option<PathBuf>,

        /// Include system information in the feedback
        #[arg(long, default_value_t = true)]
        include_system_info: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum FeedbackCategory {
    /// General feedback
    General,
    /// Bug report
    Bug,
    /// Feature request
    Feature,
    /// Performance issue
    Performance,
    /// UI/UX feedback
    Ui,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook for crash reporting
    crash_reporter::init();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::ConfigMigrate { config, backup }) => {
            cmd_config_migrate(&config, backup).await?;
        }
        Some(Commands::Feedback {
            message,
            category,
            attachment,
            include_system_info,
        }) => {
            cmd_feedback(message, category, attachment, include_system_info).await?;
        }
        None => {
            cmd_run(cli).await?;
        }
    }

    Ok(())
}

async fn cmd_run(cli: Cli) -> Result<()> {
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
        version: 1,
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

async fn cmd_config_migrate(config_path: &str, backup: bool) -> Result<()> {
    let expanded = shellexpand::tilde(config_path);
    let path = PathBuf::from(expanded.as_ref());

    if !path.exists() {
        anyhow::bail!("Configuration file not found: {}", path.display());
    }

    tracing::info!("Loading configuration from: {}", path.display());

    let data = tokio::fs::read_to_string(&path).await?;
    let config: Config = serde_json::from_str(&data)
        .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;

    // Create backup if requested
    let backup_path = if backup {
        let backup_filename = format!(
            "config-backup-{}.json",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        );
        let backup_path = path.with_file_name(backup_filename);
        tokio::fs::copy(&path, &backup_path).await?;
        tracing::info!("Backup created at: {}", backup_path.display());
        Some(backup_path)
    } else {
        None
    };

    // Run migration
    let (config, result) = migrate_config(config)?;

    if result.is_noop() {
        tracing::info!("Configuration is already up to date (v{})", result.to_version);
        println!("Configuration is already up to date (v{})", result.to_version);
        return Ok(());
    }

    tracing::info!(
        "Migrating configuration from v{} to v{}",
        result.from_version,
        result.to_version
    );

    // Save migrated config
    let migrated_data = serde_json::to_string_pretty(&config)?;
    tokio::fs::write(&path, migrated_data).await?;

    println!("Configuration migrated successfully:");
    println!("  From version: {}", result.from_version);
    println!("  To version:   {}", result.to_version);
    if let Some(ref bp) = backup_path {
        println!("  Backup:       {}", bp.display());
    }
    if !result.changes.is_empty() {
        println!("  Changes:");
        for change in &result.changes {
            println!("    - {}", change);
        }
    }

    Ok(())
}

async fn cmd_feedback(
    message: Option<String>,
    category: FeedbackCategory,
    attachment: Option<PathBuf>,
    include_system_info: bool,
) -> Result<()> {
    let feedback_dir = crash_reporter::CrashReport::default_reports_dir()
        .parent()
        .unwrap_or(&PathBuf::from("."))
        .join("feedback");
    tokio::fs::create_dir_all(&feedback_dir).await?;

    // Collect feedback message
    let message = match message {
        Some(msg) => msg,
        None => {
            println!("Please enter your feedback (press Ctrl+D when done):");
            let mut input = String::new();
            while std::io::stdin().read_line(&mut input).is_ok() {
                // Read until EOF
            }
            input.trim().to_string()
        }
    };

    if message.is_empty() {
        anyhow::bail!("Feedback message cannot be empty");
    }

    // Build feedback report
    let timestamp = chrono::Utc::now().to_rfc3339();
    let category_str = format!("{:?}", category).to_lowercase();

    let mut report = String::new();
    report.push_str("========================================\n");
    report.push_str("  ReticulumChat Beta Feedback\n");
    report.push_str("========================================\n\n");
    report.push_str(&format!("Timestamp:  {}\n", timestamp));
    report.push_str(&format!("Category:   {}\n", category_str));
    report.push_str(&format!("Version:    {}\n", env!("CARGO_PKG_VERSION")));
    report.push('\n');
    report.push_str("Feedback:\n");
    report.push_str("----------------------------------------\n");
    report.push_str(&message);
    report.push('\n');

    if include_system_info {
        report.push('\n');
        report.push_str("System Information:\n");
        report.push_str("----------------------------------------\n");
        report.push_str(&format!("OS:      {}\n", std::env::consts::OS));
        report.push_str(&format!("Arch:    {}\n", std::env::consts::ARCH));
        report.push_str(&format!("Family:  {}\n", std::env::consts::FAMILY));
    }

    // Save feedback report
    let filename = format!(
        "feedback-{}-{}.txt",
        category_str,
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let feedback_path = feedback_dir.join(&filename);
    tokio::fs::write(&feedback_path, &report).await?;

    // Handle attachment if provided
    if let Some(att_path) = attachment {
        if att_path.exists() {
            let att_filename = format!(
                "feedback-{}-{}-att-{}",
                category_str,
                chrono::Utc::now().format("%Y%m%d-%H%M%S"),
                att_path.file_name().unwrap_or_default().to_string_lossy()
            );
            let att_dest = feedback_dir.join(&att_filename);
            tokio::fs::copy(&att_path, &att_dest).await?;
            println!("Attachment saved: {}", att_dest.display());
        } else {
            tracing::warn!("Attachment not found: {}", att_path.display());
        }
    }

    println!("\nThank you for your feedback!");
    println!("Your feedback has been saved to: {}", feedback_path.display());
    println!("Please submit this file at: https://github.com/reticulumchat/reticulumchat/issues");

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
