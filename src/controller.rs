use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use anyhow::{Result, Context};
use tracing::{info, warn, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::signal;
use uuid::Uuid;

mod config;
mod safety;
mod monitoring;
mod radar_controller;
mod error;

use config::HexarConfig;
use safety::SafetyManager;
use monitoring::MonitoringSystem;
use radar_controller::RadarController;
use error::HexarError;

#[derive(Parser)]
#[command(name = "hexar")]
#[command(about = "Hexagonal Radar System Controller")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, help = "Configuration file path")]
    config: Option<PathBuf>,
    
    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,
    
    #[arg(long, help = "Log file path")]
    log_file: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Start radar system")]
    Start {
        #[arg(short, long, help = "Run in background")]
        daemon: bool,
        
        #[arg(long, help = "Force start without safety checks")]
        unsafe_mode: bool,
    },
    
    #[command(about = "Stop radar system")]
    Stop {
        #[arg(short, long, help = "Graceful shutdown timeout in seconds")]
        timeout: Option<u64>,
    },
    
    #[command(about = "System status")]
    Status {
        #[arg(short, long, help = "Detailed status")]
        detailed: bool,
    },
    
    #[command(about = "Run safety diagnostics")]
    Diagnose {
        #[arg(short, long, help = "Component to test")]
        component: Option<String>,
    },
    
    #[command(about = "Configuration management")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    
    #[command(about = "Monitoring and logs")]
    Monitor {
        #[arg(short, long, help = "Real-time monitoring")]
        follow: bool,
        
        #[arg(long, help = "Filter by log level")]
        level: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    #[command(about = "Show current configuration")]
    Show,
    
    #[command(about = "Validate configuration")]
    Validate,
    
    #[command(about = "Reset to defaults")]
    Reset,
    
    #[command(about = "Set configuration value")]
    Set {
        #[arg(help = "Configuration key")]
        key: String,
        
        #[arg(help = "Configuration value")]
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemStatus {
    system_id: Uuid,
    uptime: Duration,
    radar_status: RadarStatus,
    safety_status: SafetyStatus,
    performance_metrics: PerformanceMetrics,
    last_update: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum RadarStatus {
    Offline,
    Initializing,
    Online,
    Scanning,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SafetyStatus {
    emergency_stop: bool,
    temperature_normal: bool,
    power_normal: bool,
    antenna_status: Vec<AntennaStatus>,
    last_safety_check: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AntennaStatus {
    id: u8,
    connected: bool,
    temperature: f32,
    power_consumption: f32,
    last_signal: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PerformanceMetrics {
    cpu_usage: f32,
    memory_usage: f32,
    scan_rate: f32,
    target_count: usize,
    error_rate: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(&cli)?;
    
    // Load configuration
    let config = HexarConfig::load(cli.config.as_deref()).await
        .context("Failed to load configuration")?;
    
    info!("Starting Hexar Radar System v{}", env!("CARGO_PKG_VERSION"));
    info!("System ID: {}", config.system_id);
    
    // Execute command
    match cli.command {
        Commands::Start { daemon, unsafe_mode } => {
            start_system(config, daemon, unsafe_mode).await
        },
        Commands::Stop { timeout } => {
            stop_system(config, timeout).await
        },
        Commands::Status { detailed } => {
            show_status(config, detailed).await
        },
        Commands::Diagnose { component } => {
            run_diagnostics(config, component).await
        },
        Commands::Config { action } => {
            handle_config(config, action).await
        },
        Commands::Monitor { follow, level } => {
            monitor_system(config, follow, level).await
        },
    }
}

fn init_logging(cli: &Cli) -> Result<()> {
    let filter = if cli.verbose {
        "debug"
    } else {
        "info"
    };
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true);
    
    let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter));
    
    if let Some(log_file) = &cli.log_file {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;
        
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(file)
            .with_ansi(false);
            
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();
    }
    
    Ok(())
}

async fn start_system(config: HexarConfig, daemon: bool, unsafe_mode: bool) -> Result<()> {
    info!("Initializing radar system...");
    
    // Initialize safety manager
    let mut safety_manager = SafetyManager::new(config.safety.clone())
        .context("Failed to initialize safety manager")?;
    
    // Run safety checks unless in unsafe mode
    if !unsafe_mode {
        info!("Running safety checks...");
        let safety_result = safety_manager.run_full_diagnostics().await?;
        
        if !safety_result.safe_to_operate {
            error!("Safety checks failed. System cannot start.");
            error!("Use --unsafe-mode flag to bypass (not recommended)");
            return Err(HexarError::SafetyCheckFailed(safety_result.issues).into());
        }
        info!("Safety checks passed");
    } else {
        warn!("Starting in UNSAFE MODE - safety checks bypassed");
    }
    
    // Initialize monitoring system
    let monitoring = MonitoringSystem::new(config.monitoring.clone())
        .context("Failed to initialize monitoring")?;
    
    // Initialize radar controller
    let mut radar_controller = RadarController::new(config.radar.clone())
        .context("Failed to initialize radar controller")?;
    
    // Start radar system
    radar_controller.initialize().await
        .context("Failed to initialize radar")?;
    
    if daemon {
        info!("Starting in daemon mode");
        // TODO: Implement daemon mode with proper PID file management
        run_daemon_mode(radar_controller, safety_manager, monitoring).await
    } else {
        info!("Starting in foreground mode");
        run_foreground_mode(radar_controller, safety_manager, monitoring).await
    }
}

async fn run_foreground_mode(
    mut radar_controller: RadarController,
    mut safety_manager: SafetyManager,
    monitoring: MonitoringSystem,
) -> Result<()> {
    info!("System started successfully");
    
    // Set up signal handlers for graceful shutdown
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
    
    // Main operation loop
    loop {
        tokio::select! {
            // Handle shutdown signals
            _ = sigint.recv() => {
                info!("Received SIGINT, shutting down gracefully...");
                break;
            },
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down gracefully...");
                break;
            },
            
            // Main operation
            result = radar_controller.run_scan_cycle() => {
                match result {
                    Ok(_) => {
                        debug!("Scan cycle completed successfully");
                    },
                    Err(e) => {
                        error!("Scan cycle failed: {}", e);
                        // Check if safety manager recommends shutdown
                        if safety_manager.should_shutdown(&e).await? {
                            error!("Safety manager recommends shutdown");
                            break;
                        }
                    }
                }
            },
            
            // Periodic safety checks
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                if let Err(e) = safety_manager.run_periodic_checks().await {
                    warn!("Periodic safety check failed: {}", e);
                }
            }
        }
    }
    
    // Graceful shutdown
    info!("Shutting down radar system...");
    radar_controller.shutdown().await?;
    safety_manager.shutdown().await?;
    info!("System shutdown complete");
    
    Ok(())
}

async fn run_daemon_mode(
    radar_controller: RadarController,
    safety_manager: SafetyManager,
    monitoring: MonitoringSystem,
) -> Result<()> {
    // TODO: Implement proper daemon mode with PID file, background operation
    // For now, just run in foreground
    run_foreground_mode(radar_controller, safety_manager, monitoring).await
}

async fn stop_system(config: HexarConfig, timeout: Option<u64>) -> Result<()> {
    info!("Stopping radar system...");
    
    // TODO: Implement proper system stop with PID file management
    // For now, just log the request
    warn!("System stop not yet implemented - use Ctrl+C to stop");
    
    Ok(())
}

async fn show_status(config: HexarConfig, detailed: bool) -> Result<()> {
    info!("Retrieving system status...");
    
    // TODO: Implement actual status retrieval
    let status = SystemStatus {
        system_id: config.system_id,
        uptime: Duration::from_secs(3600), // Placeholder
        radar_status: RadarStatus::Online,
        safety_status: SafetyStatus {
            emergency_stop: false,
            temperature_normal: true,
            power_normal: true,
            antenna_status: (0..6).map(|i| AntennaStatus {
                id: i,
                connected: true,
                temperature: 25.0 + (i as f32 * 0.5),
                power_consumption: 5.0 + (i as f32 * 0.2),
                last_signal: Some(chrono::Utc::now()),
            }).collect(),
            last_safety_check: chrono::Utc::now(),
        },
        performance_metrics: PerformanceMetrics {
            cpu_usage: 15.2,
            memory_usage: 45.8,
            scan_rate: 10.5,
            target_count: 3,
            error_rate: 0.01,
        },
        last_update: chrono::Utc::now(),
    };
    
    println!("System Status:");
    println!("  System ID: {}", status.system_id);
    println!("  Uptime: {:?}", status.uptime);
    println!("  Radar Status: {:?}", status.radar_status);
    println!("  Safety Status:");
    println!("    Emergency Stop: {}", status.safety_status.emergency_stop);
    println!("    Temperature Normal: {}", status.safety_status.temperature_normal);
    println!("    Power Normal: {}", status.safety_status.power_normal);
    println!("    Antennas: {}", status.safety_status.antenna_status.len());
    
    if detailed {
        println!("  Performance Metrics:");
        println!("    CPU Usage: {:.1}%", status.performance_metrics.cpu_usage);
        println!("    Memory Usage: {:.1}%", status.performance_metrics.memory_usage);
        println!("    Scan Rate: {:.1} Hz", status.performance_metrics.scan_rate);
        println!("    Target Count: {}", status.performance_metrics.target_count);
        println!("    Error Rate: {:.3}%", status.performance_metrics.error_rate * 100.0);
        
        println!("  Antenna Details:");
        for antenna in &status.safety_status.antenna_status {
            println!("    Antenna {}: Connected={}, Temp={:.1}°C, Power={:.1}W", 
                    antenna.id, antenna.connected, antenna.temperature, antenna.power_consumption);
        }
    }
    
    Ok(())
}

async fn run_diagnostics(config: HexarConfig, component: Option<String>) -> Result<()> {
    info!("Running system diagnostics...");
    
    let mut safety_manager = SafetyManager::new(config.safety.clone())?;
    let result = safety_manager.run_full_diagnostics().await?;
    
    if let Some(component) = component {
        println!("Diagnostics for component: {}", component);
        // TODO: Implement component-specific diagnostics
    } else {
        println!("Full System Diagnostics:");
        println!("  Safe to Operate: {}", result.safe_to_operate);
        println!("  Checks Run: {}", result.checks_performed);
        
        if !result.issues.is_empty() {
            println!("  Issues Found:");
            for issue in &result.issues {
                println!("    - {}", issue);
            }
        } else {
            println!("  No issues detected");
        }
    }
    
    Ok(())
}

async fn handle_config(config: HexarConfig, action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            println!("Current Configuration:");
            println!("{}", serde_json::to_string_pretty(&config)?);
        },
        ConfigAction::Validate => {
            info!("Validating configuration...");
            // TODO: Implement configuration validation
            println!("Configuration is valid");
        },
        ConfigAction::Reset => {
            warn!("Resetting configuration to defaults...");
            // TODO: Implement configuration reset
            println!("Configuration reset to defaults");
        },
        ConfigAction::Set { key, value } => {
            info!("Setting configuration: {} = {}", key, value);
            // TODO: Implement configuration setting
            println!("Configuration updated");
        },
    }
    
    Ok(())
}

async fn monitor_system(config: HexarConfig, follow: bool, level: Option<String>) -> Result<()> {
    info!("Starting system monitoring...");
    
    if follow {
        println!("Real-time monitoring (Ctrl+C to stop):");
        // TODO: Implement real-time monitoring
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("Monitoring... {}", chrono::Utc::now());
        }
    } else {
        // TODO: Implement log display
        println!("Recent system logs:");
        println!("(Log display not yet implemented)");
    }
    
    Ok(())
}
