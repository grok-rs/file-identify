# file-identify

[![Crates.io](https://img.shields.io/crates/v/file-identify.svg)](https://crates.io/crates/file-identify)
[![docs.rs](https://docs.rs/file-identify/badge.svg)](https://docs.rs/file-identify)
[![CI](https://github.com/grok-rs/file-identify/workflows/CI/badge.svg)](https://github.com/grok-rs/file-identify/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.94.0-blue.svg)](https://blog.rust-lang.org/)

A Rust library for identifying file types based on extensions, content, and shebangs.

Given a file (or pre-loaded file information), returns a set of standardized tags
identifying what the file is. Supports 315+ file types with compile-time optimized
lookups via [PHF](https://crates.io/crates/phf).

Rust port of the Python [identify](https://github.com/pre-commit/identify) library.

## Quick start

```toml
[dependencies]
file-identify = "0.3"
```

```rust
use file_identify::{tags_from_path, tags_from_filename};

// Full identification from a filesystem path
let tags = tags_from_path("src/main.rs").unwrap();
assert!(tags.contains("file"));
assert!(tags.contains("rust"));
assert!(tags.contains("text"));

// Filename-only identification (no I/O)
let tags = tags_from_filename("Dockerfile");
assert!(tags.contains("dockerfile"));
```

## I/O-free identification

For use with mocked or virtual filesystems (e.g., in tests), `tags_from_info`
accepts pre-loaded file data with no filesystem access:

```rust
use file_identify::{tags_from_info, FileInfo, FileKind};

let info = FileInfo {
    filename: "script.py",
    file_kind: FileKind::Regular,
    is_executable: false,
    content: Some(b"print('hello')"),
};
let tags = tags_from_info(&info);
assert!(tags.contains("python"));
assert!(tags.contains("text"));
```

The `FileIdentifier` builder works with both paths and `FileInfo`:

```rust
use file_identify::FileIdentifier;

let id = FileIdentifier::new()
    .skip_content_analysis()
    .skip_shebang_analysis();

let tags = id.identify("src/main.rs").unwrap();
// Or: id.identify_from(&info);
```

## How it works

A call to `tags_from_path` does this:

1. Checks the file type (file, symlink, directory, socket). If not a regular file, stop.
2. Checks permissions and adds `executable` or `non-executable`.
3. Matches the filename or extension. If recognized, adds tags (including `text`/`binary`) and stops.
4. Reads the first 1KB to determine `text` or `binary`.
5. For text executables, parses the shebang to identify the interpreter.

By design, recognized extensions skip file reads entirely.

## CLI

The CLI is behind the `cli` feature to keep the library dependency-free of `clap`:

```bash
cargo install file-identify --features cli

file-identify src/main.rs
# ["file", "non-executable", "rust", "text"]

file-identify --filename-only Cargo.toml
# ["cargo", "text", "toml"]
```

## Tag categories

| Category | Tags |
|----------|------|
| Type | `file`, `directory`, `symlink`, `socket` |
| Mode | `executable`, `non-executable` |
| Encoding | `text`, `binary` |
| Language | `python`, `rust`, `javascript`, `c++`, ... (315+ types) |

## Minimum supported Rust version

The MSRV is **1.94.0** and is checked in CI.

## License

MIT
