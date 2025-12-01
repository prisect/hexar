use std::collections::HashMap;
use std::time::{Duration, Instant};
use nalgebra::{Vector2, Matrix2};
use log::{debug, info, warn};
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetState {
    Tracking,
    Falling,
    Lost,
    Predicted,
}

#[derive(Debug, Clone)]
pub struct TrackedTarget {
    pub id: u32,
    pub antenna_id: u8,
    pub position: Vector2<f32>,
    pub velocity: Vector2<f32>,
    pub acceleration: Vector2<f32>,
    pub state: TargetState,
    pub confidence: f32,
    pub last_update: Instant,
    pub prediction_count: u32,
    pub fall_probability: f32,
}

impl TrackedTarget {
    #[inline]
    pub fn new(id: u32, antenna_id: u8, position: Vector2<f32>) -> Self {
        Self {
            id,
            antenna_id,
            position,
            velocity: Vector2::zeros(),
            acceleration: Vector2::zeros(),
            state: TargetState::Tracking,
            confidence: 1.0,
            last_update: Instant::now(),
            prediction_count: 0,
            fall_probability: 0.0,
        }
    }

    #[inline]
    pub fn update_position(&mut self, new_position: Vector2<f32>, dt: f32) {
        if dt > 0.0 {
            let new_velocity = (new_position - self.position) / dt;
            self.acceleration = (new_velocity - self.velocity) / dt;
            self.velocity = new_velocity;
            self.position = new_position;
            self.last_update = Instant::now();
            self.prediction_count = 0;
            self.confidence = (self.confidence * 0.8 + 0.2).min(1.0);
        }
    }

    #[inline]
    pub fn predict_position(&mut self, dt: f32) -> Vector2<f32> {
        // Kinematic prediction: p = p0 + v*t + 0.5*a*t²
        self.position + self.velocity * dt + 0.5 * self.acceleration * dt * dt
    }

    #[inline]
    pub fn is_falling(&self) -> bool {
        self.fall_probability > 0.7
    }
}

#[derive(Debug)]
pub struct KalmanFilter {
    // State vector: [x, y, vx, vy, ax, ay]
    state: Vector6,
    // Covariance matrix
    covariance: Matrix6,
    // Process noise
    process_noise: Matrix6,
    // Measurement noise
    measurement_noise: Matrix2<f32>,
    // Pre-computed matrices for performance
    state_transition: Matrix6,
    measurement_matrix: Matrix2x6,
}

type Vector6 = nalgebra::SVector<f32, 6>;
type Matrix6 = nalgebra::SMatrix<f32, 6, 6>;

impl KalmanFilter {
    pub fn new(initial_position: Vector2<f32>) -> Self {
        let mut state = Vector6::zeros();
        state[0] = initial_position.x;
        state[1] = initial_position.y;

        let covariance = Matrix6::identity() * 100.0;
        let process_noise = Matrix6::identity() * 0.1;
        let measurement_noise = Matrix2::identity() * 1.0;

        // Pre-compute static matrices
        let measurement_matrix = Matrix2x6::new(
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
        );

        Self {
            state,
            covariance,
            process_noise,
            measurement_noise,
            state_transition: Matrix6::identity(),
            measurement_matrix,
        }
    }

    #[inline]
    pub fn predict(&mut self, dt: f32) {
        // Update state transition matrix F
        let f = &mut self.state_transition;
        *f = Matrix6::identity();
        f[(0, 2)] = dt;  // x += vx * dt
        f[(1, 3)] = dt;  // y += vy * dt
        f[(0, 4)] = 0.5 * dt * dt;  // x += 0.5 * ax * dt²
        f[(1, 5)] = 0.5 * dt * dt;  // y += 0.5 * ay * dt²
        f[(2, 4)] = dt;  // vx += ax * dt
        f[(3, 5)] = dt;  // vy += ay * dt

        // Predict state
        self.state = *f * self.state;
        // Predict covariance
        self.covariance = *f * self.covariance * f.transpose() + self.process_noise;
    }

    #[inline]
    pub fn update(&mut self, measurement: Vector2<f32>) {
        // Innovation
        let innovation = Vector2::new(
            measurement.x - self.state[0], 
            measurement.y - self.state[1]
        );
        
        // Innovation covariance
        let h = &self.measurement_matrix;
        let innovation_covariance = *h * self.covariance * h.transpose() + self.measurement_noise;
        
        // Kalman gain
        let kalman_gain = self.covariance * h.transpose() * innovation_covariance.try_inverse().unwrap();

        // Update state
        let state_update = kalman_gain * innovation;
        self.state[0] += state_update[0];
        self.state[1] += state_update[1];
        
        // Update covariance
        let identity = Matrix6::identity();
        self.covariance = (identity - kalman_gain * h) * self.covariance;
    }

    #[inline]
    pub fn get_position(&self) -> Vector2<f32> {
        Vector2::new(self.state[0], self.state[1])
    }

    #[inline]
    pub fn get_velocity(&self) -> Vector2<f32> {
        Vector2::new(self.state[2], self.state[3])
    }

    #[inline]
    pub fn get_acceleration(&self) -> Vector2<f32> {
        Vector2::new(self.state[4], self.state[5])
    }
}

type Matrix2x6 = nalgebra::SMatrix<f32, 2, 6>;

#[derive(Debug)]
pub struct FallDetector {
    gravity_threshold: f32,
    velocity_threshold: f32,
    acceleration_threshold: f32,
    time_window: Duration, // Kept for future use
}

impl FallDetector {
    #[inline]
    pub fn new() -> Self {
        Self {
            gravity_threshold: -9.5,  // m/s²
            velocity_threshold: 2.0,   // m/s
            acceleration_threshold: 15.0, // m/s²
            time_window: Duration::from_millis(500),
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub fn get_time_window(&self) -> Duration {
        self.time_window
    }

    #[inline]
    pub fn analyze_fall_risk(&self, target: &TrackedTarget) -> f32 {
        let mut risk_score: f32 = 0.0;

        // Check for downward acceleration (free fall)
        if target.acceleration.y < self.gravity_threshold {
            risk_score += 0.4;
        }

        // Check for high downward velocity
        if target.velocity.y < -self.velocity_threshold {
            risk_score += 0.3;
        }

        // Check for sudden acceleration changes
        let accel_magnitude = target.acceleration.norm();
        if accel_magnitude > self.acceleration_threshold {
            risk_score += 0.2;
        }

        // Check for rapid position changes
        if target.velocity.norm() > self.velocity_threshold * 2.0 {
            risk_score += 0.1;
        }

        risk_score.min(1.0)
    }

    #[inline]
    pub fn predict_fall_trajectory(&self, target: &TrackedTarget, time_steps: usize) -> SmallVec<[Vector2<f32>; 10]> {
        let mut trajectory = SmallVec::new();
        let mut position = target.position;
        let mut velocity = target.velocity;
        let gravity = Vector2::new(0.0, -9.81);
        let dt = 0.05; // 50ms time steps

        trajectory.reserve(time_steps);
        
        for _ in 0..time_steps {
            velocity += gravity * dt;
            position += velocity * dt;
            trajectory.push(position);
        }

        trajectory
    }
}

#[derive(Debug)]
pub struct MultiTargetTracker {
    targets: HashMap<u32, TrackedTarget>,
    kalman_filters: HashMap<u32, KalmanFilter>,
    fall_detector: FallDetector,
    next_target_id: u32,
    max_targets_per_antenna: usize,
    antenna_count: u8, // Kept for validation
}

impl MultiTargetTracker {
    pub fn new(antenna_count: u8) -> Self {
        Self {
            targets: HashMap::new(),
            kalman_filters: HashMap::new(),
            fall_detector: FallDetector::new(),
            next_target_id: 0,
            max_targets_per_antenna: 8,
            antenna_count,
        }
    }

    #[allow(dead_code)]
    pub fn get_antenna_count(&self) -> u8 {
        self.antenna_count
    }

    #[inline]
    pub fn add_target(&mut self, antenna_id: u8, position: Vector2<f32>) -> Option<u32> {
        // Check antenna capacity
        let current_count = self.targets.values()
            .filter(|t| t.antenna_id == antenna_id)
            .count();
        
        if current_count >= self.max_targets_per_antenna {
            warn!("Antenna {} at maximum capacity ({} targets)", antenna_id, self.max_targets_per_antenna);
            return None;
        }

        let target_id = self.next_target_id;
        self.next_target_id += 1;

        let target = TrackedTarget::new(target_id, antenna_id, position);
        let kalman_filter = KalmanFilter::new(position);

        self.targets.insert(target_id, target);
        self.kalman_filters.insert(target_id, kalman_filter);

        info!("Added target {} to antenna {} at ({:.2}, {:.2})", 
              target_id, antenna_id, position.x, position.y);

        Some(target_id)
    }

    #[inline]
    pub fn update_target(&mut self, target_id: u32, new_position: Vector2<f32>) -> bool {
        if let (Some(target), Some(kalman_filter)) = 
            (self.targets.get_mut(&target_id), self.kalman_filters.get_mut(&target_id)) {
            
            let now = Instant::now();
            let dt = (now - target.last_update).as_secs_f32();
            
            if dt > 0.0 {
                // Update Kalman filter
                kalman_filter.predict(dt);
                kalman_filter.update(new_position);
                
                // Update target with filtered values
                let filtered_pos = kalman_filter.get_position();
                target.update_position(filtered_pos, dt);
                target.velocity = kalman_filter.get_velocity();
                target.acceleration = kalman_filter.get_acceleration();
                
                // Analyze fall risk
                target.fall_probability = self.fall_detector.analyze_fall_risk(target);
                if target.fall_probability > 0.7 {
                    target.state = TargetState::Falling;
                } else {
                    target.state = TargetState::Tracking;
                }
                
                debug!("Updated target {}: pos=({:.2}, {:.2}), vel=({:.2}, {:.2}), fall_risk={:.2}", 
                       target_id, target.position.x, target.position.y, 
                       target.velocity.x, target.velocity.y, target.fall_probability);
                
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn predict_all_targets(&mut self, prediction_time: Duration) {
        let dt = prediction_time.as_secs_f32();
        
        for (target_id, target) in &mut self.targets {
            if let Some(kalman_filter) = self.kalman_filters.get_mut(target_id) {
                kalman_filter.predict(dt);
                target.position = kalman_filter.get_position();
                target.velocity = kalman_filter.get_velocity();
                target.acceleration = kalman_filter.get_acceleration();
                target.state = TargetState::Predicted;
                target.prediction_count += 1;
                target.confidence *= 0.9; // Decrease confidence with predictions
            }
        }
    }

    pub fn remove_lost_targets(&mut self, timeout: Duration) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        for (target_id, target) in &self.targets {
            if now.duration_since(target.last_update) > timeout || 
               target.confidence < 0.1 || 
               target.prediction_count > 10 {
                to_remove.push(*target_id);
            }
        }

        for target_id in to_remove {
            self.targets.remove(&target_id);
            self.kalman_filters.remove(&target_id);
            info!("Removed lost target {}", target_id);
        }
    }

    pub fn get_falling_targets(&self) -> Vec<&TrackedTarget> {
        self.targets.values()
            .filter(|t| t.is_falling())
            .collect()
    }

    pub fn get_targets_by_antenna(&self, antenna_id: u8) -> Vec<&TrackedTarget> {
        self.targets.values()
            .filter(|t| t.antenna_id == antenna_id)
            .collect()
    }

    pub fn get_target_count(&self) -> usize {
        self.targets.len()
    }

    pub fn get_target_count_by_antenna(&self, antenna_id: u8) -> usize {
        self.targets.values()
            .filter(|t| t.antenna_id == antenna_id)
            .count()
    }

    pub fn get_all_targets(&self) -> Vec<&TrackedTarget> {
        self.targets.values().collect()
    }

    #[inline]
    pub fn get_fall_predictions(&self, target_id: u32, time_steps: usize) -> Option<SmallVec<[Vector2<f32>; 10]>> {
        if let Some(target) = self.targets.get(&target_id) {
            Some(self.fall_detector.predict_fall_trajectory(target, time_steps))
        } else {
            None
        }
    }

    pub fn clear_all_targets(&mut self) {
        self.targets.clear();
        self.kalman_filters.clear();
        info!("Cleared all tracked targets");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_creation() {
        let target = TrackedTarget::new(1, 0, Vector2::new(1.0, 2.0));
        assert_eq!(target.id, 1);
        assert_eq!(target.antenna_id, 0);
        assert_eq!(target.position.x, 1.0);
        assert_eq!(target.position.y, 2.0);
    }

    #[test]
    fn test_multi_target_tracker() {
        let mut tracker = MultiTargetTracker::new(4);
        
        let target_id = tracker.add_target(0, Vector2::new(1.0, 1.0));
        assert!(target_id.is_some());
        
        assert_eq!(tracker.get_target_count(), 1);
        assert_eq!(tracker.get_target_count_by_antenna(0), 1);
    }

    #[test]
    fn test_fall_detector() {
        let detector = FallDetector::new();
        let mut target = TrackedTarget::new(1, 0, Vector2::new(0.0, 10.0));
        target.velocity = Vector2::new(0.0, -5.0);
        target.acceleration = Vector2::new(0.0, -10.0);
        
        let risk = detector.analyze_fall_risk(&target);
        assert!(risk > 0.5);
    }

    #[test]
    fn test_kalman_filter() {
        let mut kf = KalmanFilter::new(Vector2::new(0.0, 0.0));
        
        kf.predict(0.1);
        kf.update(Vector2::new(1.0, 1.0));
        
        let pos = kf.get_position();
        assert!(pos.x > 0.0);
        assert!(pos.y > 0.0);
    }
}
