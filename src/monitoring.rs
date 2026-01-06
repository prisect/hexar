use crate::config::MonitoringConfig;
use crate::error::HexarResult;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{info, warn, error, debug};
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub system_id: Uuid,
    pub performance: PerformanceMetrics,
    pub radar: RadarMetrics,
    pub safety: SafetyMetrics,
    pub errors: ErrorMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cpu_usage_percent: f32,
    pub memory_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub network_io_bytes_per_second: u64,
    pub uptime_seconds: u64,
    pub load_average: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarMetrics {
    pub scan_rate_hz: f32,
    pub targets_tracked: usize,
    pub signal_quality_db: f32,
    pub noise_floor_db: f32,
    pub antenna_status: Vec<AntennaMetrics>,
    pub processing_latency_ms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntennaMetrics {
    pub id: u8,
    pub connected: bool,
    pub temperature_celsius: f32,
    pub power_watts: f32,
    pub signal_strength_db: f32,
    pub error_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyMetrics {
    pub emergency_stop_active: bool,
    pub temperature_status: TemperatureStatus,
    pub power_status: PowerStatus,
    pub last_safety_check: chrono::DateTime<chrono::Utc>,
    pub safety_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemperatureStatus {
    Normal,
    Warning,
    Critical,
    Emergency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PowerStatus {
    Normal,
    Warning,
    Critical,
    Backup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub total_errors: u64,
    pub error_rate_per_minute: f32,
    pub recent_errors: Vec<ErrorEntry>,
    pub critical_errors: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub severity: ErrorSeverity,
    pub component: String,
    pub message: String,
    pub error_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

pub struct MonitoringSystem {
    config: MonitoringConfig,
    system_id: Uuid,
    start_time: Instant,
    metrics_history: Vec<SystemMetrics>,
    error_log: Vec<ErrorEntry>,
    alerts: Vec<Alert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub severity: AlertSeverity,
    pub category: AlertCategory,
    pub message: String,
    pub component: String,
    pub acknowledged: bool,
    pub resolved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCategory {
    System,
    Performance,
    Safety,
    Hardware,
    Software,
    Network,
}

impl MonitoringSystem {
    pub fn new(config: MonitoringConfig) -> HexarResult<Self> {
        Ok(Self {
            config,
            system_id: Uuid::new_v4(),
            start_time: Instant::now(),
            metrics_history: Vec::new(),
            error_log: Vec::new(),
            alerts: Vec::new(),
        })
    }
    
    pub async fn collect_metrics(&mut self) -> Result<SystemMetrics> {
        debug!("Collecting system metrics...");
        
        let performance = self.collect_performance_metrics().await?;
        let radar = self.collect_radar_metrics().await?;
        let safety = self.collect_safety_metrics().await?;
        let errors = self.collect_error_metrics().await?;
        
        let metrics = SystemMetrics {
            timestamp: Utc::now(),
            system_id: self.system_id,
            performance,
            radar,
            safety,
            errors,
        };
        
        // Store metrics (with retention limit)
        self.metrics_history.push(metrics.clone());
        
        let max_history = (self.config.data_retention_days * 24 * 60 * 60) / 
            self.config.health_check_interval_seconds;
        
        if self.metrics_history.len() > max_history as usize {
            self.metrics_history.remove(0);
        }
        
        // Check for alerts
        self.check_alert_conditions(&metrics).await?;
        
        Ok(metrics)
    }
    
    pub async fn log_error(&mut self, component: &str, message: &str, severity: ErrorSeverity) -> Result<()> {
        let entry = ErrorEntry {
            timestamp: Utc::now(),
            severity,
            component: component.to_string(),
            message: message.to_string(),
            error_id: Uuid::new_v4(),
        };
        
        self.error_log.push(entry.clone());
        
        // Keep error log manageable
        if self.error_log.len() > 10000 {
            self.error_log.remove(0);
        }
        
        // Create alert for critical errors
        if matches!(severity, ErrorSeverity::Critical) {
            self.create_alert(
                AlertSeverity::Critical,
                AlertCategory::Software,
                format!("Critical error in {}: {}", component, message),
                component.to_string(),
            ).await?;
        }
        
        match severity {
            ErrorSeverity::Info => debug!("[{}] {}", component, message),
            ErrorSeverity::Warning => warn!("[{}] {}", component, message),
            ErrorSeverity::Error => error!("[{}] {}", component, message),
            ErrorSeverity::Critical => error!("[CRITICAL] {}: {}", component, message),
        }
        
        Ok(())
    }
    
    pub async fn create_alert(&mut self, severity: AlertSeverity, category: AlertCategory, 
                             message: String, component: String) -> Result<()> {
        let alert = Alert {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            severity,
            category,
            message: message.clone(),
            component,
            acknowledged: false,
            resolved: false,
        };
        
        self.alerts.push(alert.clone());
        
        // Log alert
        match severity {
            AlertSeverity::Info => info!("ALERT: {}", message),
            AlertSeverity::Warning => warn!("ALERT: {}", message),
            AlertSeverity::Critical => error!("CRITICAL ALERT: {}", message),
            AlertSeverity::Emergency => error!("EMERGENCY ALERT: {}", message),
        }
        
        // TODO: Implement alert notifications (email, SMS, etc.)
        
        Ok(())
    }
    
    pub fn get_metrics_history(&self, duration: Duration) -> Vec<&SystemMetrics> {
        let cutoff = Utc::now() - chrono::Duration::from_std(duration).unwrap_or_default();
        
        self.metrics_history
            .iter()
            .filter(|m| m.timestamp > cutoff)
            .collect()
    }
    
    pub fn get_active_alerts(&self) -> Vec<&Alert> {
        self.alerts
            .iter()
            .filter(|a| !a.resolved)
            .collect()
    }
    
    pub fn acknowledge_alert(&mut self, alert_id: Uuid) -> Result<bool> {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
            info!("Alert {} acknowledged", alert_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    pub fn resolve_alert(&mut self, alert_id: Uuid) -> Result<bool> {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.resolved = true;
            info!("Alert {} resolved", alert_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    // Private helper methods
    async fn collect_performance_metrics(&self) -> Result<PerformanceMetrics> {
        // TODO: Implement actual performance monitoring
        // For now, return simulated data
        
        let uptime = self.start_time.elapsed();
        
        Ok(PerformanceMetrics {
            cpu_usage_percent: 15.2,
            memory_usage_percent: 45.8,
            disk_usage_percent: 23.1,
            network_io_bytes_per_second: 1024,
            uptime_seconds: uptime.as_secs(),
            load_average: [0.5, 0.3, 0.2],
        })
    }
    
    async fn collect_radar_metrics(&self) -> Result<RadarMetrics> {
        // TODO: Implement actual radar metrics collection
        
        let antenna_metrics = (0..6).map(|i| AntennaMetrics {
            id: i,
            connected: true,
            temperature_celsius: 25.0 + (i as f32 * 0.5),
            power_watts: 5.0 + (i as f32 * 0.2),
            signal_strength_db: -30.0 - (i as f32 * 2.0),
            error_count: 0,
        }).collect();
        
        Ok(RadarMetrics {
            scan_rate_hz: 10.5,
            targets_tracked: 3,
            signal_quality_db: -25.3,
            noise_floor_db: -85.2,
            antenna_status: antenna_metrics,
            processing_latency_ms: 15.7,
        })
    }
    
    async fn collect_safety_metrics(&self) -> Result<SafetyMetrics> {
        // TODO: Implement actual safety metrics collection
        
        Ok(SafetyMetrics {
            emergency_stop_active: false,
            temperature_status: TemperatureStatus::Normal,
            power_status: PowerStatus::Normal,
            last_safety_check: Utc::now(),
            safety_score: 0.95,
        })
    }
    
    async fn collect_error_metrics(&self) -> Result<ErrorMetrics> {
        let recent_cutoff = Utc::now() - chrono::Duration::minutes(5);
        let recent_errors: Vec<_> = self.error_log
            .iter()
            .filter(|e| e.timestamp > recent_cutoff)
            .cloned()
            .collect();
        
        let error_rate = recent_errors.len() as f32 / 5.0; // errors per minute
        
        Ok(ErrorMetrics {
            total_errors: self.error_log.len() as u64,
            error_rate_per_minute: error_rate,
            recent_errors,
            critical_errors: self.error_log.iter()
                .filter(|e| matches!(e.severity, ErrorSeverity::Critical))
                .count() as u32,
        })
    }
    
    async fn check_alert_conditions(&mut self, metrics: &SystemMetrics) -> Result<()> {
        // Check performance alerts
        if metrics.performance.cpu_usage_percent > 80.0 {
            self.create_alert(
                AlertSeverity::Warning,
                AlertCategory::Performance,
                format!("High CPU usage: {:.1}%", metrics.performance.cpu_usage_percent),
                "CPU".to_string(),
            ).await?;
        }
        
        if metrics.performance.memory_usage_percent > 90.0 {
            self.create_alert(
                AlertSeverity::Critical,
                AlertCategory::Performance,
                format!("High memory usage: {:.1}%", metrics.performance.memory_usage_percent),
                "Memory".to_string(),
            ).await?;
        }
        
        // Check radar alerts
        if metrics.radar.processing_latency_ms > 100.0 {
            self.create_alert(
                AlertSeverity::Warning,
                AlertCategory::Performance,
                format!("High processing latency: {:.1}ms", metrics.radar.processing_latency_ms),
                "Radar".to_string(),
            ).await?;
        }
        
        // Check safety alerts
        if matches!(metrics.safety.temperature_status, TemperatureStatus::Critical) {
            self.create_alert(
                AlertSeverity::Emergency,
                AlertCategory::Safety,
                "Critical temperature detected".to_string(),
                "Temperature".to_string(),
            ).await?;
        }
        
        // Check error rate alerts
        if metrics.errors.error_rate_per_minute > 10.0 {
            self.create_alert(
                AlertSeverity::Warning,
                AlertCategory::System,
                format!("High error rate: {:.1} errors/min", metrics.errors.error_rate_per_minute),
                "System".to_string(),
            ).await?;
        }
        
        Ok(())
    }
}
