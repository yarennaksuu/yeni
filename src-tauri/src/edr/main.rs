// main.rs
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod config;
mod logger;
mod policy;
mod scanner;
mod killer;

use config::Config;
use logger::Logger;
use scanner::ProcessScanner;

#[derive(Parser)]
#[command(name = "edr-kill-switch")]
#[command(about = "EDR Kill Switch - Process monitoring and termination tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Configuration file path
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,
    
    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single scan
    Scan {
        /// Dry run mode (only log, don't kill)
        #[arg(long)]
        dry_run: bool,
    },
    /// Run as daemon with periodic scanning
    Daemon {
        /// Scan interval in milliseconds
        #[arg(long, default_value = "2000")]
        interval: u64,
        
        /// Dry run mode (only log, don't kill)
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate configuration
    Validate,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize logging with clean format for Windows console
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_level))
        .with_ansi(false)  // Disable ANSI colors for Windows
        .with_target(false) // Remove target module names
        .compact()          // Use compact format
        .init();
    
    // Check for admin privileges
    let is_admin = check_admin_privileges();
    if !is_admin {
        warn!("Running without administrator privileges. Kill operations may fail.");
        println!("Warning: Running in restricted mode without admin privileges.");
        println!("Some processes cannot be terminated. Run as administrator for full functionality.");
    }
    
    // Load configuration
    let config = Config::load(&cli.config)?;
    info!("Configuration loaded from: {:?}", cli.config);
    
    // Validate configuration
    if let Commands::Validate = cli.command {
        match config.validate() {
            Ok(_) => {
                println!("Configuration is valid");
                println!("  Blacklist rules: {}", config.blacklist.len());
                println!("  Whitelist rules: {}", config.whitelist.len());
                return Ok(());
            }
            Err(e) => {
                println!("Configuration validation failed: {}", e);
                return Err(e.into());
            }
        }
    }
    
    // Initialize logger
    let logger = Logger::new(&config.logging)?;
    
    // Initialize scanner
    let mut scanner = ProcessScanner::new(config, logger);
    
    match cli.command {
        Commands::Scan { dry_run } => {
            if dry_run {
                println!("Starting scan in DRY-RUN mode (no processes will be killed)");
            }
            info!("Starting single scan (dry_run: {})", dry_run);
            scanner.scan_once(dry_run)?;
            println!("Scan completed. Check {} for details.", "edr_kill_switch.log");
        }
        Commands::Daemon { interval, dry_run } => {
            if dry_run {
                println!("Starting daemon in DRY-RUN mode (no processes will be killed)");
            }
            println!("Starting daemon mode with {}ms interval", interval);
            info!("Starting daemon mode with {}ms interval (dry_run: {})", interval, dry_run);
            scanner.run_daemon(Duration::from_millis(interval), dry_run)?;
        }
        Commands::Validate => {
            // Already handled above
        }
    }
    
    Ok(())
}

#[cfg(target_os = "windows")]
fn check_admin_privileges() -> bool {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    
    unsafe {
        let mut token_handle: HANDLE = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle).is_err() {
            return false;
        }
        
        let mut elevation = TOKEN_ELEVATION::default();
        let mut return_length = 0u32;
        
        if GetTokenInformation(
            token_handle,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        ).is_ok() {
            elevation.TokenIsElevated != 0
        } else {
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn check_admin_privileges() -> bool {
    unsafe { libc::geteuid() == 0 }
}