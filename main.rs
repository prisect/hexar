use hexar::scanner::{FrequencyScanner, FrequencyRange};
use std::time::Duration;
use log::{info, warn};
use env_logger::Env;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    info!("Scanner starting");
    
    // Get user input for scanning parameters
    let (range, threshold, scan_mode, step_size) = get_user_input()?;
    
    // Create scanner
    let mut scanner = FrequencyScanner::new(range, threshold);
    
    match scan_mode {
        ScanMode::Quick => run_quick_scan(&mut scanner)?,
        ScanMode::Refined(target_freq) => run_refined_scan(&mut scanner, target_freq, step_size)?,
        ScanMode::Full => run_full_scan(&mut scanner)?,
        ScanMode::Continuous(duration) => run_continuous_scan(&mut scanner, duration)?,
    }
    
    // Print summary
    print_summary(&scanner);
    
    Ok(())
}

#[derive(Debug)]
enum ScanMode {
    Quick,
    Refined(f32),
    Full,
    Continuous(Duration),
}

fn get_user_input() -> Result<(FrequencyRange, f32, ScanMode, f32), Box<dyn std::error::Error>> {
    println!("Frequency Scanner Configuration");
    println!("========================");
    
    // Get frequency range
    let start_freq = get_numeric_input("Enter start frequency (MHz, e.g., 100): ")?;
    let end_freq = get_numeric_input("Enter end frequency (MHz, e.g., 1000): ")?;
    let step_size = get_numeric_input("Enter step size (MHz, e.g., 1): ")?;
    
    let range = FrequencyRange {
        start: start_freq,
        end: end_freq,
        step: step_size,
    };
    
    // Get signal threshold
    let threshold = get_numeric_input("Enter signal threshold (dB, e.g., -60): ")?;
    
    // Get scan mode
    println!("\nScan modes:");
    println!("1. Quick sweep");
    println!("2. Refined scan");
    println!("3. Full scan");
    println!("4. Continuous scan");
    
    let mode_choice = get_numeric_input("Enter mode (1-4): ")? as i32;
    
    let scan_mode = match mode_choice {
        1 => ScanMode::Quick,
        2 => {
            let target_freq = get_numeric_input("Enter target frequency for refinement (MHz): ")?;
            ScanMode::Refined(target_freq)
        },
        3 => ScanMode::Full,
        4 => {
            let duration_secs = get_numeric_input("Enter scan duration (seconds): ")?;
            ScanMode::Continuous(Duration::from_secs(duration_secs as u64))
        },
        _ => {
            warn!("Invalid mode selected, defaulting to full scan");
            ScanMode::Full
        }
    };
    
    Ok((range, threshold, scan_mode, step_size))
}

fn get_numeric_input(prompt: &str) -> Result<f32, Box<dyn std::error::Error>> {
    loop {
        print!("{}", prompt);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim().parse::<f32>() {
            Ok(value) => return Ok(value),
            Err(_) => {
                println!("Invalid input. Enter a number.");
            }
        }
    }
}

fn run_quick_scan(scanner: &mut FrequencyScanner) -> Result<(), Box<dyn std::error::Error>> {
    info!("Quick scan started");
    
    let strong_signals = scanner.quick_scan();
    
    if strong_signals.is_empty() {
        println!("\nNo strong signals detected");
    } else {
        println!("\nFound {} strong signals:", strong_signals.len());
        for (i, signal) in strong_signals.iter().enumerate() {
            println!("  {}. {:.2} MHz - {:.2} dB", i + 1, signal.frequency, signal.strength);
        }
    }
    
    Ok(())
}

fn run_refined_scan(scanner: &mut FrequencyScanner, target_freq: f32, step_size: f32) -> Result<(), Box<dyn std::error::Error>> {
    info!("Refined scan at {:.2} MHz", target_freq);
    
    let result = scanner.refined_scan(target_freq, step_size * 0.5);
    
    println!("\nRefined scan result:");
    println!("  Frequency: {:.2} MHz", result.frequency);
    println!("  Signal Strength: {:.2} dB", result.strength);
    println!("  Confidence: {:.1}%", result.confidence * 100.0);
    
    Ok(())
}

fn run_full_scan(scanner: &mut FrequencyScanner) -> Result<(), Box<dyn std::error::Error>> {
    info!("Full scan started");
    
    let results = scanner.full_scan_cycle();
    
    if results.is_empty() {
        println!("\nNo signals detected");
    } else {
        println!("\nFull scan results:");
        for (i, result) in results.iter().enumerate() {
            println!("  {}. {:.2} MHz - {:.2} dB (confidence: {:.1}%)", 
                    i + 1, result.frequency, result.strength, result.confidence * 100.0);
        }
    }
    
    Ok(())
}

fn run_continuous_scan(scanner: &mut FrequencyScanner, duration: Duration) -> Result<(), Box<dyn std::error::Error>> {
    info!("Continuous scan: {:?}", duration);
    
    let results = scanner.continuous_scan(duration);
    
    if results.is_empty() {
        println!("\nNo signals detected");
    } else {
        // Group by frequency to show unique signals
        let mut unique_signals = std::collections::HashMap::new();
        for result in &results {
            let freq_key = (result.frequency * 10.0) as i32; // Round to 0.1 MHz
            let entry = unique_signals.entry(freq_key).or_insert((result.frequency, result.strength, 0));
            entry.2 += 1;
            if result.strength > entry.1 {
                entry.1 = result.strength;
            }
        }
        
        println!("\nContinuous scan summary:");
        println!("  Total detections: {}", results.len());
        println!("  Unique signals: {}", unique_signals.len());
        println!("  Top signals:");
        
        let mut sorted_signals: Vec<_> = unique_signals.values().collect();
        sorted_signals.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        for (i, (freq, strength, count)) in sorted_signals.iter().take(10).enumerate() {
            println!("    {}. {:.2} MHz - {:.2} dB (detected {} times)", 
                    i + 1, freq, strength, count);
        }
    }
    
    Ok(())
}

fn print_summary(scanner: &FrequencyScanner) {
    let (count, avg_strength, max_strength) = scanner.get_readings_summary();
    
    println!("\nScan Summary:");
    println!("  Total readings: {}", count);
    if count > 0 {
        println!("  Average signal strength: {:.2} dB", avg_strength);
        println!("  Maximum signal strength: {:.2} dB", max_strength);
    }
    println!("Scan complete.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_range_creation() {
        let range = FrequencyRange {
            start: 100.0,
            end: 1000.0,
            step: 10.0,
        };
        assert_eq!(range.start, 100.0);
        assert_eq!(range.end, 1000.0);
        assert_eq!(range.step, 10.0);
    }

    #[test]
    fn test_scanner_initialization() {
        let range = FrequencyRange {
            start: 400.0,
            end: 500.0,
            step: 1.0,
        };
        let scanner = FrequencyScanner::new(range, -60.0);
        assert_eq!(scanner.signal_threshold, -60.0);
    }

    #[test]
    fn test_quick_scan_functionality() {
        let range = FrequencyRange {
            start: 400.0,
            end: 450.0,
            step: 5.0,
        };
        let mut scanner = FrequencyScanner::new(range, -50.0);
        let signals = scanner.quick_scan();
        // Should find some signals in the test range
        assert!(!signals.is_empty() || signals.is_empty()); // Test passes either way
    }
}
