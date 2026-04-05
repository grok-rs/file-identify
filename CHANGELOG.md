# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2025-08-08

### Changed
- **Performance**: Replaced HashMap with Perfect Hash Functions (PHF) for compile-time optimization
- **Memory**: Eliminated lazy initialization overhead with static data structures  
- **Binary size**: Optimized with PHF compile-time hash generation
- **Startup**: Zero runtime hash computation for all file type lookups

### Fixed
- Improved code formatting and applied clippy suggestions for better code quality

### Technical Details
- Converted 315+ file extension mappings to PHF
- Converted 22 interpreter mappings to PHF  
- Converted 130+ special filename mappings to PHF
- Maintained 100% API compatibility and functionality
- All tests continue to pass with identical behavior

## [0.1.1] - 2025-08-08

### Changed
- **Performance**: Replaced HashMap with Perfect Hash Functions (PHF) for compile-time optimization
- **Memory**: Eliminated lazy initialization overhead with static data structures  
- **Binary size**: Optimized with PHF compile-time hash generation
- **Startup**: Zero runtime hash computation for all file type lookups

### Technical Details
- Converted 315+ file extension mappings to PHF
- Converted 22 interpreter mappings to PHF  
- Converted 130+ special filename mappings to PHF
- Maintained 100% API compatibility and functionality
- All 73 tests continue to pass with identical behavior

## [0.1.0] - 2025-08-07

### Added
- Initial Rust implementation of file identification library
- Core functionality ported from Python `identify` library
- Support for 316+ file extensions and format detection
- Shebang parsing for executable script identification
- Binary vs text content detection
- File system metadata analysis (permissions, file types)
- Command-line interface for file identification
- Comprehensive test suite with 59+ tests
- Complete API documentation with examples
- Support for Unix sockets, symlinks, and special file types

### Features
- `tags_from_path()` - comprehensive file analysis from filesystem
- `tags_from_filename()` - fast filename-only identification
- `tags_from_interpreter()` - interpreter-based script identification
- `file_is_text()` / `is_text()` - binary vs text detection
- `parse_shebang()` / `parse_shebang_from_file()` - shebang parsing
- CLI tool with JSON output and filename-only mode

### Supported File Types
- Programming languages: Python, JavaScript, Rust, Go, Java, C/C++, PHP, Ruby, Shell, etc.
- Configuration formats: JSON, YAML, TOML, XML, INI, etc.
- Documentation: Markdown, reStructuredText, AsciiDoc, etc.
- Build systems: Makefile, CMake, Bazel, Meson, etc.
- Container formats: Docker, Podman, etc.
- And 300+ more file types and extensions

[Unreleased]: https://github.com/grok-rs/file-identify/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/grok-rs/file-identify/releases/tag/v0.2.0
[0.1.1]: https://github.com/grok-rs/file-identify/releases/tag/v0.1.1
[0.1.0]: https://github.com/grok-rs/file-identify/releases/tag/v0.1.0