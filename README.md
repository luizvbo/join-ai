# Join-AI

A CLI tool to intelligently find and concatenate files into a single text file,
perfect for creating context for GenAI models like GPT and Gemini.

Tired of manually copying and pasting code into a prompt? `join-ai`
automates the process of gathering all relevant source files into one neatly
formatted file, ready for analysis.

## Features

- **Recursive File Search**: Traverses directories to find all matching files.
- **Powerful Filtering**: Include files using glob patterns (`-p "*.rs"`) and
  exclude specific folders (`-e target`), extensions, and hidden files.
- **Smart Content Detection**: Automatically detects and skips binary files to
  keep your context clean.
- **Configurable**: Control search depth, follow symlinks, and more.
- **Fast**: Built in Rust with a parallel file walker for excellent
  performance.
- **Easy to Use**: Simple and intuitive command-line interface inspired by
  modern CLI tools.

## Installation

### macOS & Linux

You can install `join-ai` with a single command. The script will download the
correct binary for your system and install it to a user-local directory.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/luizvbo/join-ai/main/install.sh | sh
```

### Windows

1.  Go to the [**Releases
    page**](https://github.com/luizvbo/join-ai/releases/latest).
2.  Download the appropriate binary for your system (e.g.,
    `join-ai-x86_64-pc-windows-msvc.exe`).
3.  Rename the file to `join-ai.exe` for convenience.
4.  Place it in a directory of your choice (for example,
    `C:\Users\YourUser\bin`).
5.  Add that directory to your system's `Path` environment variable so you can
    run `join-ai` from any terminal.

### For Rust Developers

If you have the Rust toolchain installed, you can install `join-ai` directly
from crates.io on any supported platform:

```sh
cargo install join-ai
```

## Usage

```sh
join-ai <COMMAND>
```

### Commands

-   `join`: Concatenate files from a directory into a single output file.
-   `update`: Check for new releases and update the application (not yet implemented).

### Examples

**Concatenate all files in the current directory:**

```sh
join-ai join . --output all-code.txt
```

**Concatenate only Rust (`.rs`) and Markdown (`.md`) files:**

```sh
join-ai join . -p "*.rs" -p "*.md"
```

**Check for updates:**
```sh
join-ai update
```
```

## Command-Line Options

You can get a full and up-to-date list by running `join-ai --help`.

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull
request.
