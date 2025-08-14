# Join-AI

A CLI tool to intelligently find and concatenate files into a single text file,
perfect for creating context for GenAI models like GPT and Gemini.

Tired of manually copying and pasting code into a prompt? `join-ai`
automates the process of gathering all relevant source files into one neatly
formatted file, ready for analysis.

## Features

- ✅ **Recursive File Search**: Traverses directories to find all matching
  files.
- ✅ **Powerful Filtering**: Include files using glob patterns (`-p "*.rs"`)
  and exclude specific folders (`-e target`), extensions, and hidden files.
- ✅ **Smart Content Detection**: Automatically detects and skips binary
  files to keep your context clean.
- ✅ **Configurable**: Control search depth, follow symlinks, and more.
- ✅ **Fast**: Built in Rust with a parallel file walker for excellent
  performance.
- ✅ **Easy to Use**: Simple and intuitive command-line interface inspired by
  modern CLI tools.

## Installation

Make sure you have the Rust toolchain installed. You can get it at
[rustup.rs](https://rustup.rs/).

Then, install `join-ai` directly from crates.io using Cargo:

```bash
cargo install join-ai
```

## Usage Examples

#### 1. Concatenate a Rust Project

This will find all `.rs` and `.toml` files in the current directory (`.`),
exclude the `target` and `.git` folders, and save the result to
`rust_project.txt`.

```bash
join-ai -p "*.rs" -p "*.toml" -e target -e .git -o rust_project.txt .
```

#### 2. Concatenate a Python Project

This will find all `.py` files, excluding the virtual environment folder, and
use the default output file `concatenated.txt`.

```bash
join-ai -p "*.py" -e .venv .
```

#### 3. Concatenate Web Project Files

This will find all JavaScript, HTML, and CSS files, excluding the
`node_modules` directory.

```bash
join-ai -p "*.js" -p "*.html" -p "*.css" -e node_modules .
```

## Command-Line Options

You can get a full and up-to-date list by running `join-ai --help`.

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull
request.
