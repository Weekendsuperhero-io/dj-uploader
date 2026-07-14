// Prevent an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use clap::Parser;
use dj_uploader_lib::{audio, cli, platforms};

fn main() -> Result<()> {
    let args = cli::Cli::parse();

    // No subcommand (e.g. launched from the .app bundle) or the legacy --gui
    // flag → run the Tauri desktop GUI.
    if args.gui || args.command.is_none() {
        dj_uploader_lib::run();
        return Ok(());
    }

    // CLI mode.
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    match args.command {
        Some(cli::Commands::Auth { platform }) => {
            platforms::handle_auth(platform)?;
        }
        Some(cli::Commands::Upload {
            platform,
            file,
            title,
            description,
            image,
            tags,
            publish_date,
            generate_previews,
            http1,
        }) => {
            let tag_list = tags.map(|t| {
                t.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            });

            // Parse and convert publish_date (local time) to UTC if provided.
            let publish_date_utc = if let Some(date_str) = publish_date {
                use chrono::{Local, NaiveDateTime, TimeZone};

                let naive_datetime = NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M")
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Invalid publish_date format. Use 'YYYY-MM-DD HH:MM': {}",
                            e
                        )
                    })?;

                let local_datetime = Local
                    .from_local_datetime(&naive_datetime)
                    .single()
                    .ok_or_else(|| anyhow::anyhow!("Ambiguous local time"))?;

                let utc_datetime = local_datetime.with_timezone(&chrono::Utc);
                Some(utc_datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            } else {
                None
            };

            // Generate preview snippets if requested.
            if generate_previews {
                match audio::create_preview_snippets(&file) {
                    Ok(snippets) => {
                        println!("✓ Generated {} preview snippets:", snippets.len());
                        for snippet in &snippets {
                            println!("  - {}", snippet.display());
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠ Warning: Failed to generate previews: {}", e);
                    }
                }
            }

            platforms::handle_upload(
                platform,
                &file,
                &title,
                description.as_deref(),
                image.as_deref(),
                tag_list,
                publish_date_utc.as_deref(),
                http1,
            )?;
        }
        Some(cli::Commands::Status) => {
            platforms::show_status()?;
        }
        None => unreachable!("command is Some in CLI mode"),
    }

    Ok(())
}
