use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use log::{debug, warn, error, info};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid header: expected {expected:x}, found {found:x}")]
    InvalidHeader { expected: u8, found: u8 },
    
    #[error("Length mismatch: expected {expected}, found {found}")]
    LengthMismatch { expected: usize, found: usize },
    
    #[error("Checksum failed: calculated {calc:x}, received {recv:x}")]
    ChecksumFailed { calc: u8, recv: u8 },
    
    #[error("Unknown opcode: {opcode:x}")]
    UnknownOpcode { opcode: u16 },
    
    #[error("Buffer too short: need {needed}, have {have}")]
    BufferTooShort { needed: usize, have: usize },
    
    #[error("Invalid frequency: {freq}")]
    InvalidFrequency { freq: f32 },
    
    #[error("Target data corrupted: {reason}")]
    TargetDataCorrupted { reason: String },
    
    #[error("Serial communication error: {0}")]
    SerialError(#[from] std::io::Error),
    
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub timestamp: u64,
    pub error_type: String,
    pub severity: ErrorSeverity,
    pub antenna_id: Option<u8>,
    pub target_id: Option<u32>,
    pub frequency: Option<f32>,
    pub raw_data: Option<Vec<u8>>,
    pub additional_info: HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(error_type: String, severity: ErrorSeverity) -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            error_type,
            severity,
            antenna_id: None,
            target_id: None,
            frequency: None,
            raw_data: None,
            additional_info: HashMap::new(),
        }
    }
    
    pub fn with_antenna(mut self, antenna_id: u8) -> Self {
        self.antenna_id = Some(antenna_id);
        self
    }
    
    pub fn with_target(mut self, target_id: u32) -> Self {
        self.target_id = Some(target_id);
        self
    }
    
    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = Some(frequency);
        self
    }
    
    pub fn with_raw_data(mut self, data: Vec<u8>) -> Self {
        self.raw_data = Some(data);
        self
    }
    
    pub fn with_info(mut self, key: String, value: String) -> Self {
        self.additional_info.insert(key, value);
        self
    }
}

pub struct ErrorParser {
    error_patterns: HashMap<String, ErrorPattern>,
    error_history: Vec<ErrorContext>,
    max_history: usize,
    error_counts: HashMap<String, u32>,
}

#[derive(Debug, Clone)]
struct ErrorPattern {
    name: String,
    severity: ErrorSeverity,
    description: String,
    fix_suggestion: Option<String>,
}

impl ErrorPattern {
    #[allow(dead_code)]
    pub fn get_name(&self) -> &str {
        &self.name
    }
    
    #[allow(dead_code)]
    pub fn get_description(&self) -> &str {
        &self.description
    }
}

impl ErrorParser {
    pub fn new() -> Self {
        let mut parser = Self {
            error_patterns: HashMap::new(),
            error_history: Vec::new(),
            max_history: 1000,
            error_counts: HashMap::new(),
        };
        
        parser.initialize_patterns();
        parser
    }
    
    fn initialize_patterns(&mut self) {
        // Header errors
        self.error_patterns.insert(
            "invalid_header_fd".to_string(),
            ErrorPattern {
                name: "Invalid FD Header".to_string(),
                severity: ErrorSeverity::Error,
                description: "Expected 0xFD header byte not found".to_string(),
                fix_suggestion: Some("Check serial connection and baud rate".to_string()),
            }
        );
        
        self.error_patterns.insert(
            "invalid_header_f4".to_string(),
            ErrorPattern {
                name: "Invalid F4 Header".to_string(),
                severity: ErrorSeverity::Error,
                description: "Expected 0xF4 header byte not found".to_string(),
                fix_suggestion: Some("Verify radar module is powered and connected".to_string()),
            }
        );
        
        self.error_patterns.insert(
            "invalid_header_aa".to_string(),
            ErrorPattern {
                name: "Invalid AA Header".to_string(),
                severity: ErrorSeverity::Error,
                description: "Expected 0xAA header byte not found".to_string(),
                fix_suggestion: Some("Check LD2450 module configuration".to_string()),
            }
        );
        
        // Length errors
        self.error_patterns.insert(
            "length_mismatch".to_string(),
            ErrorPattern {
                name: "Length Mismatch".to_string(),
                severity: ErrorSeverity::Warning,
                description: "Frame length doesn't match header".to_string(),
                fix_suggestion: Some("May indicate data corruption, retry reading".to_string()),
            }
        );
        
        self.error_patterns.insert(
            "buffer_too_short".to_string(),
            ErrorPattern {
                name: "Buffer Too Short".to_string(),
                severity: ErrorSeverity::Error,
                description: "Insufficient data for complete frame".to_string(),
                fix_suggestion: Some("Wait for more data or increase buffer size".to_string()),
            }
        );
        
        // Checksum errors
        self.error_patterns.insert(
            "checksum_failed".to_string(),
            ErrorPattern {
                name: "Checksum Failed".to_string(),
                severity: ErrorSeverity::Critical,
                description: "Frame checksum validation failed".to_string(),
                fix_suggestion: Some("Data corruption detected, reset connection".to_string()),
            }
        );
        
        // Target errors
        self.error_patterns.insert(
            "target_data_corrupted".to_string(),
            ErrorPattern {
                name: "Target Data Corrupted".to_string(),
                severity: ErrorSeverity::Warning,
                description: "Target tracking data appears invalid".to_string(),
                fix_suggestion: Some("Target may be lost, continue tracking".to_string()),
            }
        );
        
        // Frequency errors
        self.error_patterns.insert(
            "invalid_frequency".to_string(),
            ErrorPattern {
                name: "Invalid Frequency".to_string(),
                severity: ErrorSeverity::Error,
                description: "Frequency value out of valid range".to_string(),
                fix_suggestion: Some("Check frequency scanner configuration".to_string()),
            }
        );
        
        // Serial errors
        self.error_patterns.insert(
            "serial_error".to_string(),
            ErrorPattern {
                name: "Serial Communication Error".to_string(),
                severity: ErrorSeverity::Critical,
                description: "Serial port communication failed".to_string(),
                fix_suggestion: Some("Check cable connections and port permissions".to_string()),
            }
        );
        
        // Configuration errors
        self.error_patterns.insert(
            "configuration_error".to_string(),
            ErrorPattern {
                name: "Configuration Error".to_string(),
                severity: ErrorSeverity::Error,
                description: "Invalid configuration parameters".to_string(),
                fix_suggestion: Some("Review configuration file and parameters".to_string()),
            }
        );
    }
    
    pub fn parse_error(&mut self, error: &ParseError) -> ErrorContext {
        let error_key = self.get_error_key(error);
        let pattern = self.error_patterns.get(&error_key);
        
        let mut context = ErrorContext::new(
            error_key.clone(),
            pattern.map(|p| p.severity.clone()).unwrap_or(ErrorSeverity::Error),
        );
        
        // Extract context from error
        match error {
            ParseError::InvalidHeader { expected, found } => {
                context = context.with_info("expected".to_string(), format!("{:x}", expected));
                context = context.with_info("found".to_string(), format!("{:x}", found));
            },
            ParseError::LengthMismatch { expected, found } => {
                context = context.with_info("expected".to_string(), expected.to_string());
                context = context.with_info("found".to_string(), found.to_string());
            },
            ParseError::ChecksumFailed { calc, recv } => {
                context = context.with_info("calculated".to_string(), format!("{:x}", calc));
                context = context.with_info("received".to_string(), format!("{:x}", recv));
            },
            ParseError::UnknownOpcode { opcode } => {
                context = context.with_info("opcode".to_string(), format!("{:x}", opcode));
            },
            ParseError::BufferTooShort { needed, have } => {
                context = context.with_info("needed".to_string(), needed.to_string());
                context = context.with_info("available".to_string(), have.to_string());
            },
            ParseError::InvalidFrequency { freq } => {
                context = context.with_frequency(*freq);
            },
            ParseError::TargetDataCorrupted { reason } => {
                context = context.with_info("reason".to_string(), reason.clone());
            },
            ParseError::SerialError(source) => {
                context = context.with_info("io_error".to_string(), source.to_string());
            },
            ParseError::ConfigurationError { message } => {
                context = context.with_info("message".to_string(), message.clone());
            },
        }
        
        // Update counts
        *self.error_counts.entry(error_key).or_insert(0) += 1;
        
        // Add to history
        self.error_history.push(context.clone());
        if self.error_history.len() > self.max_history {
            self.error_history.remove(0);
        }
        
        context
    }
    
    fn get_error_key(&self, error: &ParseError) -> String {
        match error {
            ParseError::InvalidHeader { expected, .. } => {
                match *expected {
                    0xFD => "invalid_header_fd".to_string(),
                    0xF4 => "invalid_header_f4".to_string(),
                    0xAA => "invalid_header_aa".to_string(),
                    _ => "invalid_header_unknown".to_string(),
                }
            },
            ParseError::LengthMismatch { .. } => "length_mismatch".to_string(),
            ParseError::ChecksumFailed { .. } => "checksum_failed".to_string(),
            ParseError::UnknownOpcode { .. } => "unknown_opcode".to_string(),
            ParseError::BufferTooShort { .. } => "buffer_too_short".to_string(),
            ParseError::InvalidFrequency { .. } => "invalid_frequency".to_string(),
            ParseError::TargetDataCorrupted { .. } => "target_data_corrupted".to_string(),
            ParseError::SerialError { .. } => "serial_error".to_string(),
            ParseError::ConfigurationError { .. } => "configuration_error".to_string(),
        }
    }
    
    pub fn log_error(&mut self, error: &ParseError) {
        let context = self.parse_error(error);
        
        match context.severity {
            ErrorSeverity::Warning => {
                warn!("Parse warning: {} - {}", error, self.get_suggestion(&context.error_type));
            },
            ErrorSeverity::Error => {
                error!("Parse error: {} - {}", error, self.get_suggestion(&context.error_type));
            },
            ErrorSeverity::Critical => {
                error!("CRITICAL parse error: {} - {}", error, self.get_suggestion(&context.error_type));
            },
        }
        
        debug!("Error context: {:?}", context);
    }
    
    pub fn get_suggestion(&self, error_key: &str) -> String {
        self.error_patterns
            .get(error_key)
            .and_then(|p| p.fix_suggestion.clone())
            .unwrap_or_else(|| "No suggestion available".to_string())
    }
    
    pub fn get_error_summary(&self) -> HashMap<String, u32> {
        self.error_counts.clone()
    }
    
    pub fn get_recent_errors(&self, count: usize) -> Vec<&ErrorContext> {
        self.error_history
            .iter()
            .rev()
            .take(count)
            .collect()
    }
    
    pub fn get_errors_by_severity(&self, severity: ErrorSeverity) -> Vec<&ErrorContext> {
        self.error_history
            .iter()
            .filter(|ctx| ctx.severity == severity)
            .collect()
    }
    
    pub fn get_error_rate(&self, time_window_secs: u64) -> f32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let recent_errors = self.error_history
            .iter()
            .filter(|ctx| now - ctx.timestamp <= time_window_secs)
            .count();
        
        recent_errors as f32 / time_window_secs as f32
    }
    
    pub fn clear_history(&mut self) {
        self.error_history.clear();
        self.error_counts.clear();
        info!("Error parser history cleared");
    }
    
    pub fn export_errors(&self) -> String {
        let mut output = String::new();
        output.push_str("# Error Report\n\n");
        
        // Summary
        output.push_str("## Error Summary\n");
        for (error_type, count) in &self.error_counts {
            output.push_str(&format!("- {}: {}\n", error_type, count));
        }
        output.push_str("\n");
        
        // Recent errors
        output.push_str("## Recent Errors (Last 50)\n");
        for context in self.get_recent_errors(50) {
            output.push_str(&format!(
                "- [{}] {}: {}\n",
                context.timestamp,
                context.error_type,
                self.get_suggestion(&context.error_type)
            ));
        }
        
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_parsing() {
        let mut parser = ErrorParser::new();
        
        let error = ParseError::InvalidHeader { expected: 0xFD, found: 0xFF };
        let context = parser.parse_error(&error);
        
        assert_eq!(context.error_type, "invalid_header_fd");
        assert_eq!(context.severity, ErrorSeverity::Error);
    }
    
    #[test]
    fn test_error_counts() {
        let mut parser = ErrorParser::new();
        
        let error = ParseError::LengthMismatch { expected: 10, found: 5 };
        parser.log_error(&error);
        parser.log_error(&error);
        
        let summary = parser.get_error_summary();
        assert_eq!(summary.get("length_mismatch"), Some(&2));
    }
    
    #[test]
    fn test_error_rate() {
        let mut parser = ErrorParser::new();
        
        let error = ParseError::ChecksumFailed { calc: 0x12, recv: 0x34 };
        parser.log_error(&error);
        
        let rate = parser.get_error_rate(60); // 1 minute window
        assert!(rate > 0.0);
    }
}
