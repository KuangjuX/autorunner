use std::path::Path;

use anyhow::{Context, Result};

use crate::models::RunningOutput;

pub fn write_json(output: &RunningOutput, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(output).context("Failed to serialize output")?;
    std::fs::write(path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    println!("Written to {}", path.display());
    Ok(())
}

pub fn print_summary(output: &RunningOutput) {
    let s = &output.summary;
    let hours = s.total_duration_seconds / 3600;
    let minutes = (s.total_duration_seconds % 3600) / 60;

    println!("=== Running Summary ===");
    println!("Total runs:     {}", s.total_runs);
    println!("Total distance: {:.2} km", s.total_distance_km);
    println!("Avg pace:       {}/km", s.avg_pace);
    println!("Total time:     {}h {}m", hours, minutes);
    println!("Longest run:    {:.2} km", s.longest_run_km);
    println!("Activities:     {} in heatmap (past year)", output.heatmap.len());

    if let Some(ref d) = output.dashboard {
        println!("\n=== Running Level & Fitness ===");
        if let Some(level) = d.running_level {
            println!("Running level:  {:.1}", level);
        }
        if let Some(rhr) = d.resting_hr {
            println!("Resting HR:     {} bpm", rhr);
        }
        if let Some(lthr) = d.threshold_hr {
            println!("Threshold HR:   {} bpm", lthr);
        }
        if let Some(ref pace) = d.threshold_pace {
            println!("Threshold pace: {}/km", pace);
        }
        if let Some(pct) = d.recovery_pct {
            println!("Recovery:       {:.0}%", pct);
        }

        if !d.race_predictions.is_empty() {
            println!("\n=== Race Predictions ===");
            for r in &d.race_predictions {
                let h = r.duration_seconds / 3600;
                let m = (r.duration_seconds % 3600) / 60;
                let s = r.duration_seconds % 60;
                let time = if h > 0 {
                    format!("{}:{:02}:{:02}", h, m, s)
                } else {
                    format!("{}:{:02}", m, s)
                };
                println!("  {:15} {}  ({})/km", r.race, time, r.avg_pace);
            }
        }
    }

    if !output.personal_bests.is_empty() {
        println!("\n=== Personal Bests ===");
        for pb in &output.personal_bests {
            println!("  {:15} {}  ({})/km  [{}]", pb.distance, pb.time, pb.pace, pb.date);
        }
    }

    println!("\nLast synced:    {}", output.last_synced);
}
