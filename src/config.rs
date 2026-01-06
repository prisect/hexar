use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;
use uuid::Uuid;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexarConfig {
    pub system_id: Uuid,
    pub radar: RadarConfig,
    pub safety: SafetyConfig,
    pub monitoring: MonitoringConfig,
    pub logging: LoggingConfig,
}

impl HexarConfig {
    pub async fn load(path: Option<&std::path::Path>) -> Result<Self> {
        let config_path = path.unwrap_or_else(|| std::path::Path::new("config.toml"));
        
        if config_path.exists() {
            let content = tokio::fs::read_to_string(config_path).await?;
            let config: HexarConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            info!("No configuration file found, using defaults");
            Ok(HexarConfig::default())
        }
    }
    
    pub async fn save(&self, path: Option<&std::path::Path>) -> Result<()> {
        let config_path = path.unwrap_or_else(|| std::path::Path::new("config.toml"));
        
        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(config_path, content).await?;
        
        Ok(())
    }
}

impl Default for HexarConfig {
    fn default() -> Self {
        Self {
            system_id: Uuid::new_v4(),
            radar: RadarConfig::default(),
            safety: SafetyConfig::default(),
            monitoring: MonitoringConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarConfig {
    pub antenna_count: u8,
    pub default_frequency: f32,
    pub frequency_range: FrequencyRange,
    pub scan_mode: ScanMode,
    pub power_settings: PowerSettings,
    pub signal_processing: SignalProcessingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencyRange {
    pub start_mhz: f32,
    pub end_mhz: f32,
    pub step_mhz: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanMode {
    Continuous,
    Intermittent,
    OnDemand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSettings {
    pub transmit_power_watts: f32,
    pub duty_cycle: f32,
    pub power_saving: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalProcessingConfig {
    pub threshold_db: f32,
    pub filter_strength: f32,
    pub noise_reduction: bool,
    pub target_tracking: bool,
}

impl Default for RadarConfig {
    fn default() -> Self {
        Self {
            antenna_count: 6,
            default_frequency: 24000.0, // 24 GHz
            frequency_range: FrequencyRange {
                start_mhz: 24000.0,
                end_mhz: 24500.0,
                step_mhz: 1.0,
            },
            scan_mode: ScanMode::Continuous,
            power_settings: PowerSettings {
                transmit_power_watts: 10.0,
                duty_cycle: 0.8,
                power_saving: false,
            },
            signal_processing: SignalProcessingConfig {
                threshold_db: -60.0,
                filter_strength: 0.7,
                noise_reduction: true,
                target_tracking: true,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub emergency_stop_enabled: bool,
    pub temperature_limits: TemperatureLimits,
    pub power_limits: PowerLimits,
    pub radiation_limits: RadiationLimits,
    pub auto_shutdown: AutoShutdownConfig,
    pub maintenance_schedule: MaintenanceSchedule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureLimits {
    pub warning_celsius: f32,
    pub critical_celsius: f32,
    pub shutdown_celsius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerLimits {
    pub max_power_watts: f32,
    pub surge_protection: bool,
    pub voltage_tolerance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadiationLimits {
    pub max_exposure_time_minutes: u32,
    pub power_density_limit: f32,
    pub distance_requirement_meters: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoShutdownConfig {
    pub enabled: bool,
    pub idle_timeout_minutes: u32,
    pub error_threshold: u32,
    pub performance_degradation_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceSchedule {
    pub inspection_interval_hours: u32,
    pub calibration_interval_hours: u32,
    pub cleaning_interval_hours: u32,
    pub last_maintenance: chrono::DateTime<chrono::Utc>,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            emergency_stop_enabled: true,
            temperature_limits: TemperatureLimits {
                warning_celsius: 70.0,
                critical_celsius: 85.0,
                shutdown_celsius: 95.0,
            },
            power_limits: PowerLimits {
                max_power_watts: 100.0,
                surge_protection: true,
                voltage_tolerance: 0.1,
            },
            radiation_limits: RadiationLimits {
                max_exposure_time_minutes: 60,
                power_density_limit: 10.0,
                distance_requirement_meters: 3.0,
            },
            auto_shutdown: AutoShutdownConfig {
                enabled: true,
                idle_timeout_minutes: 30,
                error_threshold: 10,
                performance_degradation_threshold: 0.8,
            },
            maintenance_schedule: MaintenanceSchedule {
                inspection_interval_hours: 168, // 1 week
                calibration_interval_hours: 720, // 1 month
                cleaning_interval_hours: 336, // 2 weeks
                last_maintenance: chrono::Utc::now(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_collection: bool,
    pub performance_tracking: bool,
    pub alert_system: bool,
    pub data_retention_days: u32,
    pub export_interval_minutes: u32,
    pub health_check_interval_seconds: u32,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_collection: true,
            performance_tracking: true,
            alert_system: true,
            data_retention_days: 30,
            export_interval_minutes: 15,
            health_check_interval_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_logging: bool,
    pub console_logging: bool,
    pub log_directory: PathBuf,
    pub max_file_size_mb: u32,
    pub max_files: u32,
    pub rotation: LogRotation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogRotation {
    Daily,
    Weekly,
    Size,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_logging: true,
            console_logging: true,
            log_directory: PathBuf::from("logs"),
            max_file_size_mb: 100,
            max_files: 10,
            rotation: LogRotation::Daily,
        }
    }
}
