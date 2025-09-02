use crate::cli::JoinArgs;
use ignore::{WalkBuilder, WalkState};
use std::path::PathBuf;
use std::sync::mpsc;

/// This module is responsible for efficiently finding all files that match the
/// user's criteria using the `ignore` crate, which is excellent at respecting
/// rules like `.gitignore` and handling parallel directory traversal.
///
/// The walker runs in a separate thread pool and sends valid file paths back to the
/// main thread through a multi-producer, single-consumer (mpsc) channel.
///
/// # Arguments
/// * `args` - A reference to the parsed `JoinArgs` containing all CLI options.
///
/// # Returns
/// A `Result` containing the receiver end of the channel, which will be used by
/// the processor to receive file paths.
pub fn find_files(args: &JoinArgs) -> anyhow::Result<mpsc::Receiver<PathBuf>> {
    // Create a channel for communication between the walker threads and the main thread.
    let (tx, rx) = mpsc::channel();
    let input_folder = args.input_folder.clone();

    // --- 1. Configure the base walker ---
    let mut walker_builder = WalkBuilder::new(&input_folder);
    walker_builder
        .follow_links(!args.no_follow)
        .max_depth(args.max_depth);

    // --- 2. Build a set of override rules for inclusion and exclusion ---
    // The `OverrideBuilder` allows us to programmatically add glob patterns that
    // take precedence over any `.gitignore` or similar rules.
    let mut override_builder = ignore::overrides::OverrideBuilder::new(&input_folder);

    // Add inclusion patterns. If none are provided, default to including everything.
    if let Some(patterns) = &args.patterns {
        for pattern in patterns {
            override_builder.add(pattern)?;
        }
    } else {
        // A single "*" will match all files, which is a good default.
        override_builder.add("*")?;
    }

    // Add all exclusion patterns. These are prefixed with "!" to negate the match.
    if let Some(exclude_patterns) = &args.exclude {
        for pattern in exclude_patterns {
            let exclusion_pattern = format!("!{pattern}");
            override_builder.add(&exclusion_pattern)?;
        }
    }

    // If hidden files are not requested, add a global ignore pattern for them.
    // This is necessary because the `*` override would otherwise include them.
    if !args.hidden {
        override_builder.add("!.*")?;
    }

    // Apply the built override rules to the walker.
    let overrides = override_builder.build()?;
    walker_builder.overrides(overrides);

    // --- 3. Run the walker in parallel ---
    let walker = walker_builder.build_parallel();
    let output_file_path = args.output_file.clone();

    // The `run` method spawns a thread pool to perform the walk.
    // We provide a closure that builds a "move closure" for each thread.
    walker.run(move || {
        // Clone the transmitter and other necessary data for each thread.
        let tx = tx.clone();
        let output_file_path = output_file_path.clone();

        // This inner closure is executed for each directory entry found.
        Box::new(move |result| {
            if let Ok(entry) = result {
                let path = entry.path();
                // Skip directories and the application's own output file.
                if path.is_dir() || path == output_file_path {
                    return WalkState::Continue;
                }

                // All filtering is now handled by the `overrides`, so we don't
                // need to manually check extensions or folders here.

                // If all checks pass, send the valid file path to the processor.
                tx.send(path.to_path_buf()).expect("Failed to send path");
            }
            // Continue the walk regardless of the result.
            WalkState::Continue
        })
    });

    // Return the receiver end of the channel to the caller.
    Ok(rx)
}
