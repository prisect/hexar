use std::time::{Duration, Instant};
use log::{info, warn, debug};

#[derive(Debug, Clone)]
pub struct FrequencyRange {
    pub start: f32,
    pub end: f32,
    pub step: f32,
}

#[derive(Debug, Clone)]
pub struct SignalReading {
    pub frequency: f32,
    pub strength: f32,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub frequency: f32,
    pub strength: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct FrequencyScanner {
    current_range: FrequencyRange,
    signal_threshold: f32,
    max_refinement_iterations: usize,
    readings: Vec<SignalReading>,
}

impl FrequencyScanner {
    pub fn new(initial_range: FrequencyRange, signal_threshold: f32) -> Self {
        Self {
            current_range: initial_range,
            signal_threshold,
            max_refinement_iterations: 5,
            readings: Vec::new(),
        }
    }

    pub fn set_threshold(&mut self, threshold: f32) {
        self.signal_threshold = threshold;
    }

    pub fn scan_frequency(&mut self, frequency: f32) -> SignalReading {
        // Simulate reading signal strength at given frequency
        let strength = self.simulate_signal_reading(frequency);
        let reading = SignalReading {
            frequency,
            strength,
            timestamp: Instant::now(),
        };
        
        self.readings.push(reading.clone());
        debug!("Frequency {:.2} MHz: Signal strength {:.2} dB", frequency, strength);
        reading
    }

    fn simulate_signal_reading(&self, frequency: f32) -> f32 {
        // Simulate signal with noise and occasional strong signals
        let base_noise = -80.0; // Base noise floor in dB
        let noise_variation = (frequency * 0.1).sin() * 5.0; // Some frequency-dependent variation
        
        // Add occasional strong signals at specific frequencies
        let signal_boost = if (frequency - 433.0).abs() < 2.0 {
            40.0 + (frequency - 433.0).abs() * 10.0 // Strong signal around 433 MHz
        } else if (frequency - 915.0).abs() < 5.0 {
            35.0 + (frequency - 915.0).abs() * 5.0 // Another signal around 915 MHz
        } else if (frequency - 2400.0).abs() < 10.0 {
            30.0 + (frequency - 2400.0).abs() * 2.0 // WiFi band
        } else {
            0.0
        };
        
        base_noise + noise_variation + signal_boost + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as f32 * 0.000000001).sin() * 2.0
    }

    pub fn quick_scan(&mut self) -> Vec<SignalReading> {
        info!("Quick scan: {:.1} to {:.1} MHz", 
              self.current_range.start, self.current_range.end);
        
        let mut strong_signals = Vec::new();
        let mut freq = self.current_range.start;
        
        while freq <= self.current_range.end {
            let reading = self.scan_frequency(freq);
            if reading.strength > self.signal_threshold {
                info!("Signal at {:.2} MHz: {:.2} dB", freq, reading.strength);
                strong_signals.push(reading);
            }
            freq += self.current_range.step;
        }
        
        strong_signals
    }

    pub fn refined_scan(&mut self, target_frequency: f32, initial_step: f32) -> ScanResult {
        info!("Refined scan at {:.2} MHz", target_frequency);
        
        let mut best_frequency = target_frequency;
        let mut best_strength = self.scan_frequency(target_frequency).strength;
        let mut current_step = initial_step;
        let mut iteration = 0;
        
        while current_step > 0.01 && iteration < self.max_refinement_iterations {
            debug!("Refinement iteration {}: step = {:.3} MHz", iteration, current_step);
            
            // Check frequencies around the current best
            let test_frequencies = [
                best_frequency - current_step,
                best_frequency + current_step,
            ];
            
            let mut found_better = false;
            
            for &freq in &test_frequencies {
                if freq >= self.current_range.start && freq <= self.current_range.end {
                    let reading = self.scan_frequency(freq);
                    if reading.strength > best_strength {
                        best_strength = reading.strength;
                        best_frequency = freq;
                        found_better = true;
                        debug!("Better signal at {:.2} MHz: {:.2} dB", freq, reading.strength);
                    }
                }
            }
            
            if !found_better {
                // Reduce step size for finer search
                current_step *= 0.5;
                debug!("No better signal, step: {:.3} MHz", current_step);
            }
            
            iteration += 1;
            
            // Add small delay to simulate real scanning
            std::thread::sleep(Duration::from_millis(10));
        }
        
        // Calculate confidence based on signal strength and stability
        let confidence = self.calculate_confidence(best_frequency, best_strength);
        
        info!("Refined: {:.2} MHz, {:.2} dB, {:.1}% confidence", 
              best_frequency, best_strength, confidence * 100.0);
        
        ScanResult {
            frequency: best_frequency,
            strength: best_strength,
            confidence,
        }
    }

    fn calculate_confidence(&self, frequency: f32, strength: f32) -> f32 {
        // Get recent readings around this frequency
        let recent_readings: Vec<_> = self.readings
            .iter()
            .filter(|r| (r.frequency - frequency).abs() < 1.0)
            .collect();
        
        if recent_readings.len() < 3 {
            return 0.5; // Low confidence with few samples
        }
        
        // Calculate stability (lower variance = higher confidence)
        let mean_strength = recent_readings.iter().map(|r| r.strength).sum::<f32>() / recent_readings.len() as f32;
        let variance = recent_readings.iter()
            .map(|r| (r.strength - mean_strength).powi(2))
            .sum::<f32>() / recent_readings.len() as f32;
        
        let stability_factor = 1.0 / (1.0 + variance);
        
        // Signal strength factor (stronger signals are more reliable)
        let strength_factor = (strength / 100.0).min(1.0).max(0.0);
        
        // Combine factors
        (stability_factor * 0.6 + strength_factor * 0.4).min(1.0)
    }

    pub fn full_scan_cycle(&mut self) -> Vec<ScanResult> {
        info!("Full scan cycle started");
        
        // Phase 1: Quick scan to find strong signals
        let strong_signals = self.quick_scan();
        
        if strong_signals.is_empty() {
            warn!("No signals above threshold detected");
            return Vec::new();
        }
        
        // Phase 2: Refine around each strong signal
        let mut results = Vec::new();
        for signal in &strong_signals {
            let refined = self.refined_scan(signal.frequency, self.current_range.step * 0.5);
            results.push(refined);
        }
        
        // Sort by strength (strongest first)
        results.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap());
        
        info!("Scan complete: {} signals found", results.len());
        results
    }

    pub fn continuous_scan(&mut self, duration: Duration) -> Vec<ScanResult> {
        info!("Continuous scan: {:?}", duration);
        
        let start_time = Instant::now();
        let mut all_results = Vec::new();
        
        while start_time.elapsed() < duration {
            let cycle_results = self.full_scan_cycle();
            all_results.extend(cycle_results);
            
            // Small delay between cycles
            std::thread::sleep(Duration::from_millis(100));
        }
        
        info!("Continuous scan complete: {} detections", all_results.len());
        all_results
    }

    pub fn get_readings_summary(&self) -> (usize, f32, f32) {
        if self.readings.is_empty() {
            return (0, 0.0, 0.0);
        }
        
        let count = self.readings.len();
        let avg_strength = self.readings.iter().map(|r| r.strength).sum::<f32>() / count as f32;
        let max_strength = self.readings.iter().map(|r| r.strength).fold(f32::MIN, f32::max);
        
        (count, avg_strength, max_strength)
    }

    pub fn clear_readings(&mut self) {
        self.readings.clear();
        debug!("Readings cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_scanner_creation() {
        let range = FrequencyRange {
            start: 100.0,
            end: 1000.0,
            step: 10.0,
        };
        let scanner = FrequencyScanner::new(range, -50.0);
        assert_eq!(scanner.signal_threshold, -50.0);
    }

    #[test]
    fn test_signal_reading() {
        let range = FrequencyRange {
            start: 400.0,
            end: 500.0,
            step: 1.0,
        };
        let mut scanner = FrequencyScanner::new(range, -50.0);
        let reading = scanner.scan_frequency(433.0);
        assert!(reading.strength > -100.0); // Should be above noise floor
        assert_eq!(reading.frequency, 433.0);
    }

    #[test]
    fn test_quick_scan() {
        let range = FrequencyRange {
            start: 400.0,
            end: 500.0,
            step: 10.0,
        };
        let mut scanner = FrequencyScanner::new(range, -60.0);
        let signals = scanner.quick_scan();
        // Should find some signals in the 433 MHz range
        assert!(!signals.is_empty());
    }

    #[test]
    fn test_refined_scan() {
        let range = FrequencyRange {
            start: 400.0,
            end: 500.0,
            step: 1.0,
        };
        let mut scanner = FrequencyScanner::new(range, -60.0);
        let result = scanner.refined_scan(433.0, 1.0);
        assert!(result.frequency >= 400.0 && result.frequency <= 500.0);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }
}
