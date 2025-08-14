use crate::cli::Args;
use ignore::{WalkBuilder, WalkState};
use std::path::PathBuf;
use std::sync::mpsc;

/// Sets up and runs the file walker, sending valid file paths through a channel.
pub fn find_files(args: &Args) -> anyhow::Result<mpsc::Receiver<PathBuf>> {
    let (tx, rx) = mpsc::channel();
    let input_folder = args.input_folder.clone();

    let mut walker_builder = WalkBuilder::new(&input_folder);
    walker_builder
        .hidden(!args.hidden)
        .follow_links(!args.no_follow)
        .max_depth(args.max_depth);

    let mut override_builder = ignore::overrides::OverrideBuilder::new(&input_folder);

    if let Some(patterns) = &args.patterns {
        for pattern in patterns {
            override_builder.add(pattern)?;
        }
    } else {
        override_builder.add("*")?; // Default to including all files if no pattern is given
    }

    if let Some(exclude_folders) = &args.exclude_folders {
        for folder in exclude_folders {
            // The "!" prefix negates the pattern, effectively excluding it.
            override_builder.add(&format!("!{}", folder))?;
        }
    }

    let overrides = override_builder.build()?;
    walker_builder.overrides(overrides);

    let walker = walker_builder.build_parallel();
    let output_file_path = args.output_file.clone();
    let exclude_extensions = args.exclude_extensions.clone();

    // The walker runs in a separate thread pool
    walker.run(move || {
        let tx = tx.clone();
        let exclude_extensions = exclude_extensions.clone();
        let output_file_path = output_file_path.clone();

        Box::new(move |result| {
            if let Ok(entry) = result {
                let path = entry.path();
                // Skip directories and the output file itself
                if path.is_dir() || path == output_file_path {
                    return WalkState::Continue;
                }

                // Filter by extension
                if let Some(ext_str) = path.extension().and_then(|s| s.to_str())
                    && let Some(exts_to_exclude) = &exclude_extensions
                    && exts_to_exclude.contains(&ext_str.to_string())
                {
                    return WalkState::Continue;
                }

                // If all checks pass, send the path for processing
                tx.send(path.to_path_buf()).expect("Failed to send path");
            }
            WalkState::Continue
        })
    });

    Ok(rx)
}
