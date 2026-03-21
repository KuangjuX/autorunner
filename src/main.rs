mod client;
mod models;
mod output;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "autorunner", about = "Sync running data from COROS watches")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch running data from COROS and output as JSON
    Sync {
        /// COROS account email (or set COROS_ACCOUNT env var)
        #[arg(long, env = "COROS_ACCOUNT")]
        account: String,

        /// COROS account password (or set COROS_PASSWORD env var)
        #[arg(long, env = "COROS_PASSWORD")]
        password: String,

        /// Output JSON file path
        #[arg(short, long, default_value = "running_data.json")]
        output: PathBuf,
    },

    /// Print summary from an existing JSON file
    Summary {
        /// Input JSON file path
        #[arg(short, long, default_value = "running_data.json")]
        input: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Sync {
            account,
            password,
            output: output_path,
        } => {
            let password_md5 = format!("{:x}", md5::compute(password.as_bytes()));

            println!("Logging in to COROS...");
            let coros = client::CorosClient::login(&account, &password_md5)
                .await
                .context("COROS login failed")?;

            println!("Fetching running activities...");
            let raw_activities = coros.fetch_all_running_activities().await?;

            println!("Fetching dashboard (running level, race predictor)...");
            let dashboard = match coros.fetch_dashboard().await {
                Ok(d) => Some(d),
                Err(e) => {
                    eprintln!("Warning: failed to fetch dashboard: {e}");
                    None
                }
            };

            println!("Processing data...");
            let mut running_output = models::build_output(&raw_activities, dashboard);

            let highlights = models::select_highlights(&running_output.activities);
            if !highlights.is_empty() {
                println!(
                    "Fetching route maps for {} highlight activities...",
                    highlights.len()
                );
                for (tag, idx) in &highlights {
                    let activity = &running_output.activities[*idx];
                    let label_id = activity.label_id.clone();
                    let sport_type = activity.raw_sport_type;
                    print!("  [{tag}] {} {} ... ", activity.date, activity.distance_km);

                    match coros.fetch_activity_route(&label_id, sport_type).await {
                        Ok(points) if !points.is_empty() => {
                            println!("{} GPS points", points.len());
                            running_output
                                .highlight_routes
                                .push(models::HighlightRoute {
                                    tag: tag.clone(),
                                    date: activity.date.clone(),
                                    distance_km: activity.distance_km,
                                    duration_seconds: activity.duration_seconds,
                                    pace_per_km: activity.pace_per_km.clone(),
                                    sport_type: activity.sport_type.clone(),
                                    points,
                                });
                        }
                        Ok(_) => println!("no GPS data"),
                        Err(e) => println!("failed: {e}"),
                    }
                }
            }

            output::write_json(&running_output, &output_path)?;
            output::print_summary(&running_output);

            Ok(())
        }
        Commands::Summary { input } => {
            let content = std::fs::read_to_string(&input)
                .with_context(|| format!("Failed to read {}", input.display()))?;
            let running_output: models::RunningOutput =
                serde_json::from_str(&content).context("Failed to parse JSON")?;
            output::print_summary(&running_output);
            Ok(())
        }
    }
}
