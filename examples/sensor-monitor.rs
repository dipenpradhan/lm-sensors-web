//! # lm-sensors-web Example
//!
//! Demonstrates using `lm-sensors-web` as a library — not as a binary.
//! Shows sensor reading, config loading, and real-time monitoring.
//!
//! ## Run
//!
//! ```bash
//! cargo run --example sensor-monitor
//! ```
//!
//! ## Usage
//!
//! ```bash
//! # Monitor sensors every 5 seconds (default)
//! cargo run --example sensor-monitor
//!
//! # Monitor with custom interval (seconds)
//! cargo run --example sensor-monitor -- --interval 2
//!
//! # Filter devices by name
//! cargo run --example sensor-monitor -- --filter temp
//! ```

use clap::Parser;
use std::time::Duration;

/// A simple sensor monitoring CLI tool using lm-sensors-web as a library.
#[derive(Parser, Debug)]
#[command(
    name = "sensor-monitor",
    about = "Real-time sensor monitor using lm-sensors-web"
)]
struct Args {
    /// Poll interval in seconds
    #[arg(short, long, default_value_t = 5)]
    interval: u64,

    /// Filter devices by name (case-insensitive substring match)
    #[arg(short, long)]
    filter: Option<String>,
}

fn main() {
    let args = Args::parse();
    let interval = Duration::from_secs(args.interval);

    println!("lm-sensors-web example: sensor-monitor");
    println!("Press Ctrl+C to stop\n");

    // ── 1. Initialize SensorManager (core library entry point) ─────────
    let sensor_manager = lm_sensors_web::sensors::SensorManager::new()
        .expect("Failed to initialize lm-sensors library (is libsensors installed?)");

    // ── 2. List devices (metadata only, fast, no sensor reads) ─────────
    let devices = sensor_manager.list_devices();
    println!("Found {} sensor device(s):", devices.len());
    for device in &devices {
        println!(
            "  • {} (bus: {}, path: {:?})",
            device.name, device.bus, device.path
        );
    }

    // ── 3. Load default config ─────────────────────────────────────────
    let config = lm_sensors_web::config::Config::default();
    println!("\nDefault config:");
    println!("  • Host:   {}", config.server.host);
    println!("  • Port:   {}", config.server.port);
    println!("  • Log:    {}", config.server.log_level);
    println!(
        "  • WS:     enabled={}, interval={}ms",
        config.websocket.enabled, config.websocket.broadcast_interval_ms
    );
    println!("  • Webhooks: {} configured", config.webhooks.len());

    println!("\nMonitoring every {}s (Ctrl+C to stop):\n", args.interval);

    // ── 4. Monitoring loop ─────────────────────────────────────────────
    loop {
        let readings = sensor_manager.read_all();
        let now = chrono::Local::now();

        println!("╔══════════════════════════════════════════════════════╗");
        println!(
            "║  Sensor Readings — {}                         ║",
            now.format("%Y-%m-%d %H:%M:%S")
        );
        println!("╚══════════════════════════════════════════════════════╝");

        let mut temp_min: Option<f64> = None;
        let mut temp_max: Option<f64> = None;

        for dev_readings in &readings.devices {
            // Apply filter if provided
            if let Some(ref filter_str) = args.filter
                && !dev_readings
                    .device
                    .name
                    .to_lowercase()
                    .contains(&filter_str.to_lowercase())
            {
                continue;
            }

            println!("\n  📦  {}", dev_readings.device.name);

            for feature in &dev_readings.features {
                for sub in &feature.sub_features {
                    if let Some(value) = sub.value {
                        let unit_str = sub.unit.as_deref().unwrap_or("");
                        let display = format!(
                            "{:.1}{}",
                            value,
                            if !unit_str.is_empty() { unit_str } else { "" }
                        );
                        println!("     → {:20} = {}", sub.name, display);

                        // Track temperature stats
                        if sub.name.to_lowercase().contains("temp") {
                            temp_min = Some(match temp_min {
                                Some(min) => min.min(value),
                                None => value,
                            });
                            temp_max = Some(match temp_max {
                                Some(max) => max.max(value),
                                None => value,
                            });
                        }
                    }
                }
            }
        }

        // Print summary
        if let (Some(min), Some(max)) = (temp_min, temp_max) {
            println!("\n  🌡️  Temperature range: {:.1}°C — {:.1}°C", min, max);
        }

        std::thread::sleep(interval);
    }
}
