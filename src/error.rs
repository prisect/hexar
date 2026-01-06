use thiserror::Error;

#[derive(Error, Debug)]
pub enum HexarError {
    #[error("Safety check failed: {0:?}")]
    SafetyCheckFailed(Vec<String>),
    
    #[error("Radar initialization failed: {0}")]
    RadarInitializationFailed(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Hardware error: {0}")]
    HardwareError(String),
    
    #[error("Communication error: {0}")]
    CommunicationError(String),
    
    #[error("Signal processing error: {0}")]
    SignalProcessingError(String),
    
    #[error("Monitoring error: {0}")]
    MonitoringError(String),
    
    #[error("System error: {0}")]
    SystemError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Configuration parsing error: {0}")]
    ConfigParseError(#[from] toml::de::Error),
    
    #[error("Time error: {0}")]
    TimeError(#[from] chrono::ParseError),
    
    #[error("UUID error: {0}")]
    UuidError(#[from] uuid::Error),
    
    #[error("Operation cancelled")]
    OperationCancelled,
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Resource unavailable: {0}")]
    ResourceUnavailable(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Timeout occurred: {0}")]
    Timeout(String),
}

pub type HexarResult<T> = Result<T, HexarError>;
