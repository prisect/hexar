use crate::tracker::MultiTargetTracker;
use nalgebra::Vector2;
use std::thread;
use std::time::Duration;

pub fn run_multi_target_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("Multi-Target Tracking Demo");
    println!("=========================");
    
    // Create tracker for 4 antennas
    let mut tracker = MultiTargetTracker::new(4);
    
    // Simulate targets appearing and moving
    println!("\nAdding targets to antennas...");
    
    // Add targets to different antennas
    let _target_id = tracker.add_target(0, Vector2::new(1.0, 1.0));
    let target2 = tracker.add_target(1, Vector2::new(-1.0, 3.0));
    let target3 = tracker.add_target(0, Vector2::new(2.0, 1.0));
    let _target4 = tracker.add_target(2, Vector2::new(0.0, -2.0));
    
    println!("Added {} targets", tracker.get_target_count());
    
    // Simulate movement and falling
    println!("\nSimulating target movement...");
    
    for step in 0..50 {
        // Update target positions
        if let Some(id) = target2 {
            let new_pos = Vector2::new(-1.0 + step as f32 * 0.05, 3.0);
            tracker.update_target(id, new_pos);
        }
        
        // Target 3 starts falling after step 20
        if let Some(id) = target3 {
            let mut new_pos = Vector2::new(2.0, 1.0);
            if step > 20 {
                new_pos.y = 1.0 - (step - 20) as f32 * 0.3; // Falling
            }
            tracker.update_target(id, new_pos);
        }
        
        // Check for falling targets
        let falling_targets = tracker.get_falling_targets();
        if !falling_targets.is_empty() && step % 10 == 0 {
            println!("Step {}: {} falling targets detected", step, falling_targets.len());
            for target in falling_targets {
                println!("  Target {} on antenna {} - Fall probability: {:.2}", 
                        target.id, target.antenna_id, target.fall_probability);
                
                // Show fall prediction
                if let Some(trajectory) = tracker.get_fall_predictions(target.id, 5) {
                    println!("    Predicted trajectory: {:?}", 
                            trajectory.iter().map(|p| format!("({:.1},{:.1})", p.x, p.y)).collect::<Vec<_>>());
                }
            }
        }
        
        // Remove lost targets periodically
        if step % 25 == 0 {
            tracker.remove_lost_targets(Duration::from_secs(2));
        }
        
        thread::sleep(Duration::from_millis(100));
    }
    
    // Show final status
    println!("\nFinal tracker status:");
    println!("Total targets: {}", tracker.get_target_count());
    
    for antenna_id in 0..4 {
        let antenna_count = tracker.get_target_count_by_antenna(antenna_id);
        if antenna_count > 0 {
            println!("Antenna {}: {} targets", antenna_id, antenna_count);
            
            let targets = tracker.get_targets_by_antenna(antenna_id);
            for target in targets {
                println!("  Target {}: pos=({:.2},{:.2}), state={:?}, confidence={:.2}", 
                        target.id, target.position.x, target.position.y, 
                        target.state, target.confidence);
            }
        }
    }
    
    Ok(())
}

pub fn run_stress_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("Multi-Target Stress Test");
    println!("=======================");
    
    let mut tracker = MultiTargetTracker::new(8); // 8 antennas
    
    // Try to add maximum targets per antenna
    println!("\nTesting antenna capacity...");
    
    for antenna in 0..8 {
        println!("Antenna {}: ", antenna);
        
        for i in 0..10 { // Try to add 10 targets (max is 8)
            let pos = Vector2::new(i as f32 * 0.5, antenna as f32 * 0.5);
            if let Some(_target_id) = tracker.add_target(antenna, pos) {
                print!("✓");
            } else {
                print!("✗");
            }
        }
        println!(" ({}/8)", tracker.get_target_count_by_antenna(antenna));
    }
    
    println!("Total targets: {}", tracker.get_target_count());
    
    // Simulate rapid updates
    println!("\nTesting rapid updates...");
    let start_time = std::time::Instant::now();
    
    for step in 0..1000 {
        let target_ids: Vec<u32> = tracker.get_all_targets().iter().map(|t| t.id).collect();
        for target_id in target_ids {
            let noise = Vector2::new(
                (step as f32 * 0.01).sin() * 0.1,
                (step as f32 * 0.01).cos() * 0.1
            );
            if let Some(target) = tracker.get_all_targets().iter().find(|t| t.id == target_id) {
                tracker.update_target(target.id, target.position + noise);
            }
        }
    }
    
    let elapsed = start_time.elapsed();
    println!("1000 update cycles completed in {:?}", elapsed);
    
    Ok(())
}
