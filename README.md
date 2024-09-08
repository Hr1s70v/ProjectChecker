# ProjectChecker

**ProjectChecker** is a Rust-based tool designed to analyze public GitHub repositories. It provides comprehensive information about the repository, including a tree structure of the repository, and a detailed count of various file types.

## Features

- **Tree Structure**: Shows a hierarchical view of the repository's files and directories.
- **File Type Analysis**: Lists and counts all file types present in the repository.

## Getting Started

### Prerequisites

- Rust (Version 1.65 or higher)
- A GitHub account (for accessing public repositories)

### Installation

Clone the repository and build the project using Cargo:

```bash
git clone https://github.com/yourusername/ProjectChecker.git
cd ProjectChecker
cargo build --release
```
## Usage
Run the application and provide the URL of the GitHub repository when prompted:
You will be prompted to enter the repository URL interactively.
```bash
https://github.com/owner/rep
```

Example Output:
```bash
Tree URL: https://api.github.com/repos/Hr1s70v/ProjectChecker/git/trees/master?recursive=1
.gitignore
Cargo.lock
Cargo.toml
extensions.json
src/
  src/api.rs
  src/display.rs
  src/lib.rs
  src/main.rs
src/api.rs
src/display.rs
src/lib.rs
src/main.rs
Matched file type: Rust for file: src/lib.rs
Matched file type: JSON for file: extensions.json
Matched file type: Package Managers for file: Cargo.toml
Matched file type: Git for file: .gitignore
Matched file type: Rust for file: src/display.rs
Matched file type: Rust for file: src/api.rs
Matched file type: Rust for file: src/main.rs
Matched file type: Package Managers for file: Cargo.lock
Repository contents:
--------------------------------------------------
File Type: Package Managers
Files: 2
--------------------------------------------------
File Type: Rust
Files: 4
--------------------------------------------------
File Type: JSON
Files: 1
--------------------------------------------------
File Type: Git
Files: 1
--------------------------------------------------
```

## License
This project does not have a specific license. Use and contribute to the project at your own discretion.