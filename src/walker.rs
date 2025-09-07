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
    let input_folder = args.input_folder.clone();

    // --- 1. Build a set of override rules for inclusion and exclusion ---
    let mut override_builder = ignore::overrides::OverrideBuilder::new(&input_folder);
    if let Some(patterns) = &args.patterns {
        for pattern in patterns {
            override_builder.add(pattern)?;
        }
    } else {
        override_builder.add("*")?;
    }
    if let Some(exclude_patterns) = &args.exclude {
        for pattern in exclude_patterns {
            let exclusion_pattern = format!("!{pattern}");
            override_builder.add(&exclusion_pattern)?;
        }
    }
    if !args.hidden {
        override_builder.add("!.*")?;
    }
    let overrides = override_builder.build()?;

    // --- 2. If verbosity is >= 2, run a diagnostic walk ---
    if args.verbose > 1 {
        println!("\n[Verbose Mode] Analyzing file matches...");
        let debug_walker = WalkBuilder::new(&input_folder)
            .follow_links(!args.no_follow)
            .max_depth(args.max_depth)
            .hidden(!args.hidden) // Walker's hidden(true) means "ignore hidden"
            .build();

        for result in debug_walker {
            let entry = match result {
                Ok(entry) => entry,
                Err(e) => {
                    eprintln!("[Verbose Mode] Error: {}", e);
                    continue;
                }
            };
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            if path == args.output_file {
                println!(
                    "- {:<10} {} (is the output file)",
                    "Skipped",
                    path.display()
                );
                continue;
            }

            match overrides.matched(path, false) {
                ignore::Match::Whitelist(glob) => {
                    println!(
                        "+ {:<10} {} (matched include pattern: '{:?}')",
                        "Included",
                        path.display(),
                        glob
                    );
                }
                ignore::Match::Ignore(glob) => {
                    println!(
                        "- {:<10} {} (matched exclude pattern: '{:?}')",
                        "Excluded",
                        path.display(),
                        glob
                    );
                }
                ignore::Match::None => {
                    // The walker already filters out files ignored by .gitignore etc.
                    // So if we are here, the file was not ignored by standard rules.
                    if args.patterns.is_some() {
                        println!(
                            "- {:<10} {} (did not match any include pattern)",
                            "Excluded",
                            path.display()
                        );
                    } else {
                        println!(
                            "+ {:<10} {} (included by default)",
                            "Included",
                            path.display()
                        );
                    }
                }
            }
        }
        println!("[Verbose Mode] Analysis complete.\n");
    }

    // --- 3. Run the main, parallel walker for actual processing ---
    let (tx, rx) = mpsc::channel();
    let mut walker_builder = WalkBuilder::new(&input_folder);
    walker_builder
        .follow_links(!args.no_follow)
        .max_depth(args.max_depth);
    walker_builder.overrides(overrides.clone()); // Use the same rules

    let walker = walker_builder.build_parallel();
    let output_file_path = args.output_file.clone();

    walker.run(move || {
        let tx = tx.clone();
        let output_file_path = output_file_path.clone();
        Box::new(move |result| {
            if let Ok(entry) = result {
                let path = entry.path();
                if path.is_dir() || path == output_file_path {
                    return WalkState::Continue;
                }
                tx.send(path.to_path_buf()).expect("Failed to send path");
            }
            WalkState::Continue
        })
    });

    Ok(rx)
}
