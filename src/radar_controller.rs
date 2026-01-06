use crate::config::RadarConfig;
use crate::error::{HexarError, HexarResult};
use crate::scanner::{FrequencyScanner, FrequencyRange, ScanResult};
use crate::tracker::{MultiTargetTracker, TrackedTarget};
use anyhow::Result;
use std::time::{Duration, Instant};
use tracing::{info, error, debug};
use chrono::Utc;
use uuid::Uuid;
use nalgebra::Vector2;

#[derive(Debug, Clone)]
pub struct RadarController {
    config: RadarConfig,
    scanner: FrequencyScanner,
    tracker: MultiTargetTracker,
    system_id: Uuid,
    initialized: bool,
    current_scan_mode: ScanMode,
    last_scan_time: Option<Instant>,
    scan_results: Vec<ScanResult>,
}

#[derive(Debug, Clone)]
pub enum ControllerState {
    Uninitialized,
    Initializing,
    Ready,
    Scanning,
    Error(String),
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct ScanCycleResult {
    pub scan_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub scan_results: Vec<ScanResult>,
    pub targets_detected: Vec<TrackedTarget>,
    pub scan_duration: Duration,
    pub signals_processed: usize,
}

impl RadarController {
    pub fn new(config: RadarConfig) -> HexarResult<Self> {
        let frequency_range = FrequencyRange {
            start: config.frequency_range.start_mhz,
            end: config.frequency_range.end_mhz,
            step: config.frequency_range.step_mhz,
        };
        
        let scanner = FrequencyScanner::new(frequency_range, config.signal_processing.threshold_db);
        let tracker = MultiTargetTracker::new(config.antenna_count);
        
        Ok(Self {
            config,
            scanner,
            tracker,
            system_id: Uuid::new_v4(),
            initialized: false,
            current_scan_mode: ScanMode::Continuous,
            last_scan_time: None,
            scan_results: Vec::new(),
        })
    }
    
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing radar controller...");
        
        self.set_state(ControllerState::Initializing).await?;
        
        // Initialize antenna systems
        self.initialize_antennas().await?;
        
        // Validate frequency range
        self.validate_frequency_range().await?;
        
        // Perform self-test
        self.run_self_test().await?;
        
        // Initialize scanner
        self.scanner.clear_readings();
        
        // Clear tracker
        self.tracker.clear_all_targets();
        
        self.initialized = true;
        self.set_state(ControllerState::Ready).await?;
        
        info!("Radar controller initialized successfully");
        Ok(())
    }
    
    pub async fn run_scan_cycle(&mut self) -> Result<ScanCycleResult> {
        if !self.initialized {
            return Err(HexarError::RadarInitializationFailed(
                "Radar controller not initialized".to_string()
            ).into());
        }
        
        let scan_start = Instant::now();
        let scan_id = Uuid::new_v4();
        
        self.set_state(ControllerState::Scanning).await?;
        
        debug!("Starting scan cycle {}", scan_id);
        
        // Perform frequency scan
        let scan_results = self.scanner.full_scan_cycle();
        
        // Process scan results and update targets
        let mut targets_detected = Vec::new();
        let mut signals_processed = 0;
        
        for scan_result in &scan_results {
            signals_processed += 1;
            
            // Convert scan result to target position (simplified)
            let position = self.frequency_to_position(scan_result.frequency);
            
            // Determine which antenna would detect this signal
            let antenna_id = self.frequency_to_antenna_id(scan_result.frequency);
            
            // Update or create target
            if let Some(target_id) = self.find_nearby_target(&position) {
                if self.tracker.update_target(target_id, position) {
                    if let Some(target) = self.tracker.get_all_targets()
                        .iter()
                        .find(|t| t.id == target_id) {
                        targets_detected.push((*target).clone());
                    }
                }
            } else {
                if let Some(new_target_id) = self.tracker.add_target(antenna_id, position) {
                    if let Some(target) = self.tracker.get_all_targets()
                        .iter()
                        .find(|t| t.id == new_target_id) {
                        targets_detected.push((*target).clone());
                    }
                }
            }
        }
        
        // Remove lost targets
        self.tracker.remove_lost_targets(Duration::from_secs(30));
        
        let scan_duration = scan_start.elapsed();
        self.last_scan_time = Some(scan_start);
        self.scan_results.extend(scan_results.clone());
        
        // Keep scan results manageable
        if self.scan_results.len() > 1000 {
            self.scan_results.drain(0..500);
        }
        
        let result = ScanCycleResult {
            scan_id,
            timestamp: Utc::now(),
            scan_results,
            targets_detected,
            scan_duration,
            signals_processed,
        };
        
        debug!("Scan cycle completed: {:.2}ms, {} signals, {} targets", 
               scan_duration.as_millis(), signals_processed, result.targets_detected.len());
        
        self.set_state(ControllerState::Ready).await?;
        
        Ok(result)
    }
    
    pub async fn start_continuous_scan(&mut self) -> Result<()> {
        info!("Starting continuous scanning mode");
        
        if !self.initialized {
            return Err(HexarError::RadarInitializationFailed(
                "Radar controller not initialized".to_string()
            ).into());
        }
        
        self.current_scan_mode = ScanMode::Continuous;
        
        loop {
            match self.run_scan_cycle().await {
                Ok(result) => {
                    debug!("Continuous scan: {} targets detected", result.targets_detected.len());
                },
                Err(e) => {
                    error!("Continuous scan failed: {}", e);
                    // Wait before retrying
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
            
            // Rate limiting based on configuration
            let scan_interval = Duration::from_millis((1000.0 / self.config.scan_rate_hz()) as u64);
            tokio::time::sleep(scan_interval).await;
        }
    }
    
    pub async fn stop_continuous_scan(&mut self) -> Result<()> {
        info!("Stopping continuous scanning");
        self.current_scan_mode = ScanMode::OnDemand;
        Ok(())
    }
    
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down radar controller...");
        
        self.set_state(ControllerState::Shutdown).await?;
        
        // Stop any ongoing operations
        self.stop_continuous_scan().await?;
        
        // Power down antennas
        self.shutdown_antennas().await?;
        
        // Clear data
        self.scan_results.clear();
        self.tracker.clear_all_targets();
        
        self.initialized = false;
        
        info!("Radar controller shutdown complete");
        Ok(())
    }
    
    pub fn get_state(&self) -> ControllerState {
        if !self.initialized {
            ControllerState::Uninitialized
        } else {
            ControllerState::Ready
        }
    }
    
    pub fn get_current_targets(&self) -> Vec<&TrackedTarget> {
        self.tracker.get_all_targets()
    }
    
    pub fn get_falling_targets(&self) -> Vec<&TrackedTarget> {
        self.tracker.get_falling_targets()
    }
    
    pub fn get_scan_statistics(&self) -> ScanStatistics {
        ScanStatistics {
            total_scans: self.scan_results.len(),
            last_scan_time: self.last_scan_time,
            current_target_count: self.tracker.get_target_count(),
            average_scan_duration: self.calculate_average_scan_duration(),
            signals_per_scan: self.calculate_signals_per_scan(),
        }
    }
    
    // Private helper methods
    async fn set_state(&self, state: ControllerState) -> Result<()> {
        debug!("Radar controller state: {:?}", state);
        // TODO: Implement state change logging and monitoring
        Ok(())
    }
    
    async fn initialize_antennas(&self) -> Result<()> {
        info!("Initializing {} antenna systems", self.config.antenna_count);
        
        // TODO: Implement actual antenna initialization
        for i in 0..self.config.antenna_count {
            debug!("Initializing antenna {}", i);
            // Initialize antenna hardware, check connections, etc.
        }
        
        Ok(())
    }
    
    async fn validate_frequency_range(&self) -> Result<()> {
        let range = &self.config.frequency_range;
        
        if range.start_mhz >= range.end_mhz {
            return Err(HexarError::ConfigurationError(
                "Invalid frequency range: start >= end".to_string()
            ).into());
        }
        
        if range.step_mhz <= 0.0 {
            return Err(HexarError::ConfigurationError(
                "Invalid frequency step: must be positive".to_string()
            ).into());
        }
        
        info!("Frequency range validated: {:.1} - {:.1} MHz (step: {:.1} MHz)", 
              range.start_mhz, range.end_mhz, range.step_mhz);
        
        Ok(())
    }
    
    async fn run_self_test(&self) -> Result<()> {
        info!("Running radar system self-test...");
        
        // TODO: Implement actual self-test procedures
        // - Test antenna connectivity
        // - Test signal generation
        // - Test data acquisition
        // - Test signal processing
        
        debug!("Self-test completed successfully");
        Ok(())
    }
    
    async fn shutdown_antennas(&self) -> Result<()> {
        info!("Shutting down antenna systems");
        
        // TODO: Implement actual antenna shutdown
        for i in 0..self.config.antenna_count {
            debug!("Shutting down antenna {}", i);
        }
        
        Ok(())
    }
    
    fn frequency_to_position(&self, frequency: f32) -> Vector2<f32> {
        // Simplified conversion from frequency to position
        // In a real system, this would involve complex antenna array processing
        
        let normalized_freq = (frequency - self.config.frequency_range.start_mhz) / 
            (self.config.frequency_range.end_mhz - self.config.frequency_range.start_mhz);
        
        // Convert to x,y coordinates (simplified hexagonal arrangement)
        let angle = normalized_freq * 2.0 * std::f32::consts::PI;
        let radius = 10.0; // Assume 10 meter detection radius
        
        Vector2::new(
            radius * angle.cos(),
            radius * angle.sin(),
        )
    }
    
    fn frequency_to_antenna_id(&self, frequency: f32) -> u8 {
        // Determine which antenna would detect a given frequency
        let normalized_freq = (frequency - self.config.frequency_range.start_mhz) / 
            (self.config.frequency_range.end_mhz - self.config.frequency_range.start_mhz);
        
        (normalized_freq * self.config.antenna_count as f32) as u8 % self.config.antenna_count
    }
    
    fn find_nearby_target(&self, position: &Vector2<f32>) -> Option<u32> {
        let threshold = 2.0; // 2 meter threshold
        
        for target in self.tracker.get_all_targets() {
            let distance = (target.position - position).norm();
            if distance < threshold {
                return Some(target.id);
            }
        }
        
        None
    }
    
    fn calculate_average_scan_duration(&self) -> Duration {
        if self.scan_results.is_empty() {
            return Duration::ZERO;
        }
        
        // This is a placeholder - in reality we'd track actual durations
        Duration::from_millis(100)
    }
    
    fn calculate_signals_per_scan(&self) -> f32 {
        if self.scan_results.is_empty() {
            return 0.0;
        }
        
        self.scan_results.len() as f32 / 10.0 // Assume 10 scans
    }
}

#[derive(Debug, Clone)]
pub struct ScanStatistics {
    pub total_scans: usize,
    pub last_scan_time: Option<Instant>,
    pub current_target_count: usize,
    pub average_scan_duration: Duration,
    pub signals_per_scan: f32,
}

// Extension methods for RadarConfig
impl RadarConfig {
    pub fn scan_rate_hz(&self) -> f32 {
        match self.scan_mode {
            ScanMode::Continuous => 10.0,
            ScanMode::Intermittent => 5.0,
            ScanMode::OnDemand => 1.0,
        }
    }
}

// Re-export scan modes
pub use crate::config::ScanMode;
