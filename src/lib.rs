//! # file-identify
//!
//! A Rust library for identifying file types based on extensions, content, and shebangs.
//!
//! This library provides a comprehensive way to identify files by analyzing:
//! - File extensions and special filenames
//! - File content (binary vs text detection)
//! - Shebang lines for executable scripts
//! - File system metadata (permissions, file type)
//!
//! ## Quick Start
//!
//! ```rust
//! use file_identify::{tags_from_path, tags_from_filename, FileIdentifier};
//!
//! // Simple filename identification
//! let tags = tags_from_filename("script.py");
//! assert!(tags.contains("python"));
//! assert!(tags.contains("text"));
//!
//! // Full file identification from filesystem path
//! # use std::fs;
//! # use tempfile::tempdir;
//! # let dir = tempdir().unwrap();
//! # let file_path = dir.path().join("test.py");
//! # fs::write(&file_path, "print('hello')").unwrap();
//! let tags = tags_from_path(&file_path).unwrap();
//! assert!(tags.contains("file"));
//! assert!(tags.contains("python"));
//!
//! // Customized identification with builder pattern
//! let identifier = FileIdentifier::new()
//!     .skip_content_analysis()  // Skip text vs binary detection
//!     .skip_shebang_analysis(); // Skip shebang parsing
//!
//! let tags = identifier.identify(&file_path).unwrap();
//! assert!(tags.contains("file"));
//! assert!(tags.contains("python"));
//! ```
//!
//! ## Tag System
//!
//! Files are identified using a set of standardized tags:
//!
//! - **Type tags**: `file`, `directory`, `symlink`, `socket`
//! - **Mode tags**: `executable`, `non-executable`
//! - **Encoding tags**: `text`, `binary`
//! - **Language/format tags**: `python`, `javascript`, `json`, `xml`, etc.
//!
//! ## Error Handling
//!
//! Functions that access the filesystem return [`Result`] types. The main error
//! conditions are:
//!
//! - [`IdentifyError::PathNotFound`] - when the specified path doesn't exist
//! - [`IdentifyError::IoError`] - for other I/O related errors

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io::{BufReader, Read};
use std::path::Path;

pub mod extensions;
pub mod interpreters;
pub mod tags;

/// A tuple-like immutable container for shebang components that matches Python's tuple behavior.
///
/// This type is designed to be a direct equivalent to Python's `tuple[str, ...]` for
/// parse_shebang functions, providing immutable access to shebang components.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShebangTuple {
    components: Box<[String]>,
}

impl ShebangTuple {
    /// Create a new empty ShebangTuple (equivalent to Python's `()`)
    pub fn new() -> Self {
        Self {
            components: Box::new([]),
        }
    }

    /// Create a ShebangTuple from a vector of strings
    pub fn from_vec(vec: Vec<String>) -> Self {
        Self {
            components: vec.into_boxed_slice(),
        }
    }

    /// Get the length of the tuple (equivalent to Python's `len(tuple)`)
    pub const fn len(&self) -> usize {
        self.components.len()
    }

    /// Check if the tuple is empty (equivalent to Python's `not tuple`)
    pub const fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Get an element by index (equivalent to Python's `tuple[index]`)
    /// Returns None if index is out of bounds
    pub fn get(&self, index: usize) -> Option<&str> {
        self.components.get(index).map(|s| s.as_str())
    }

    /// Get the first element (equivalent to Python's `tuple[0]` when safe)
    pub fn first(&self) -> Option<&str> {
        self.get(0)
    }

    /// Convert to a Vec for internal use (consumes the tuple)
    pub fn into_vec(self) -> Vec<String> {
        self.components.into_vec()
    }

    /// Iterate over the components (equivalent to Python's `for item in tuple`)
    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.components.iter()
    }

    /// Convert to a slice for easy pattern matching
    pub fn as_slice(&self) -> &[String] {
        &self.components
    }
}

// Implement Index trait for tuple[index] syntax
impl std::ops::Index<usize> for ShebangTuple {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        &self.components[index]
    }
}

// Implement IntoIterator for for-loops
impl<'a> IntoIterator for &'a ShebangTuple {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.components.iter()
    }
}

// Implement FromIterator for collecting
impl FromIterator<String> for ShebangTuple {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        Self::from_vec(iter.into_iter().collect())
    }
}

// Display implementation (equivalent to Python's str(tuple))
impl fmt::Display for ShebangTuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for (i, component) in self.components.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "'{component}'")?;
        }
        if self.components.len() == 1 {
            write!(f, ",")?; // Python tuple trailing comma for single element
        }
        write!(f, ")")
    }
}

// Conversion from Vec<String>
impl From<Vec<String>> for ShebangTuple {
    fn from(vec: Vec<String>) -> Self {
        Self::from_vec(vec)
    }
}

// Conversion from empty ()
impl Default for ShebangTuple {
    fn default() -> Self {
        Self::new()
    }
}

use extensions::{get_extension_tags, get_extensions_need_binary_check_tags, get_name_tags};
use interpreters::get_interpreter_tags;
pub use tags::FileKind;
use tags::*;

/// Pre-loaded file information for I/O-free identification.
///
/// Use this when you have file data from a source other than the host
/// filesystem (e.g., a mocked/virtual filesystem in tests).
///
/// # Examples
///
/// ```rust
/// use file_identify::{tags_from_info, FileInfo, FileKind};
///
/// let info = FileInfo {
///     filename: "script.py",
///     file_kind: FileKind::Regular,
///     is_executable: false,
///     content: Some(b"print('hello')"),
/// };
/// let tags = tags_from_info(&info);
/// assert!(tags.contains("python"));
/// assert!(tags.contains("text"));
/// ```
#[derive(Debug, Clone)]
pub struct FileInfo<'a> {
    /// The filename (just the name component, not a full path).
    pub filename: &'a str,
    /// The kind of filesystem entry.
    pub file_kind: FileKind,
    /// Whether the file has executable permissions.
    pub is_executable: bool,
    /// Optional file content for shebang and encoding analysis.
    /// Pass `None` to skip content-based analysis entirely.
    pub content: Option<&'a [u8]>,
}

/// Configuration for file identification behavior.
///
/// Allows customizing which analysis steps to perform and their order.
/// Use `FileIdentifier::new()` to create a builder and customize identification.
#[derive(Debug, Clone)]
pub struct FileIdentifier {
    skip_content_analysis: bool,
    skip_shebang_analysis: bool,
    custom_extensions: Option<std::collections::HashMap<String, TagSet>>,
}

impl Default for FileIdentifier {
    fn default() -> Self {
        Self::new()
    }
}

impl FileIdentifier {
    /// Create a new file identifier with default settings.
    ///
    /// By default, all analysis steps are enabled:
    /// - File system metadata analysis
    /// - Filename and extension analysis  
    /// - Shebang analysis for executable files
    /// - Content analysis (text vs binary detection)
    pub fn new() -> Self {
        Self {
            skip_content_analysis: false,
            skip_shebang_analysis: false,
            custom_extensions: None,
        }
    }

    /// Skip content analysis (text vs binary detection).
    ///
    /// This avoids reading file contents, making identification faster
    /// but potentially less accurate for files without clear extension/filename patterns.
    pub fn skip_content_analysis(mut self) -> Self {
        self.skip_content_analysis = true;
        self
    }

    /// Skip shebang analysis for executable files.
    ///
    /// This avoids parsing shebang lines, making identification faster
    /// but less accurate for executable scripts without recognized extensions.
    pub fn skip_shebang_analysis(mut self) -> Self {
        self.skip_shebang_analysis = true;
        self
    }

    /// Add custom file extension mappings.
    ///
    /// These will be checked before the built-in extension mappings.
    /// Useful for organization-specific or project-specific file types.
    pub fn with_custom_extensions(
        mut self,
        extensions: std::collections::HashMap<String, TagSet>,
    ) -> Self {
        self.custom_extensions = Some(extensions);
        self
    }

    /// Identify a file using the configured settings.
    ///
    /// This is equivalent to `tags_from_path` but with customizable behavior.
    pub fn identify<P: AsRef<Path>>(&self, path: P) -> Result<TagSet> {
        self.identify_with_config(path)
    }

    fn identify_with_config<P: AsRef<Path>>(&self, path: P) -> Result<TagSet> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();

        // Get file metadata
        let metadata = match fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(_) => {
                return Err(IdentifyError::PathNotFound {
                    path: path_str.to_string(),
                });
            }
        };

        // Step 1: Check for non-regular file types (directory, symlink, socket)
        if let Some(file_type_tags) = analyze_file_type(&metadata) {
            return Ok(file_type_tags);
        }

        // Step 2: This is a regular file - start building tag set
        let mut tags = TagSet::new();
        tags.insert(FILE);

        // Step 3: Analyze permissions (executable vs non-executable)
        let is_executable = analyze_permissions(path, &metadata);
        tags.insert(if is_executable {
            EXECUTABLE
        } else {
            NON_EXECUTABLE
        });

        // Step 4: Analyze filename and potentially shebang (with custom config)
        tags.extend(self.analyze_filename_and_shebang_configured(path, is_executable));

        // Step 5: Analyze content encoding (text vs binary) if not skipped and not already determined
        if !self.skip_content_analysis {
            tags.extend(analyze_content_encoding(path, &tags)?);
        }

        Ok(tags)
    }

    /// Identify a file from pre-loaded information, using configured settings.
    ///
    /// This is the pure, I/O-free equivalent of [`FileIdentifier::identify`].
    /// The caller provides all necessary file data via [`FileInfo`].
    pub fn identify_from(&self, info: &FileInfo<'_>) -> TagSet {
        match info.file_kind {
            FileKind::Directory => return HashSet::from([DIRECTORY]),
            FileKind::Symlink => return HashSet::from([SYMLINK]),
            FileKind::Socket => return HashSet::from([SOCKET]),
            FileKind::Regular => {}
        }

        let mut tags = TagSet::new();
        tags.insert(FILE);
        tags.insert(if info.is_executable {
            EXECUTABLE
        } else {
            NON_EXECUTABLE
        });

        // Filename analysis with custom extensions support
        let mut filename_matched = false;
        if let Some(custom_exts) = &self.custom_extensions
            && let Some(ext) = Path::new(info.filename)
                .extension()
                .and_then(|e| e.to_str())
            && let Some(ext_tags) = custom_exts.get(&ext.to_lowercase())
        {
            tags.extend(ext_tags.iter().copied());
            filename_matched = true;
        }
        if !filename_matched {
            let filename_tags = tags_from_filename(info.filename);
            if !filename_tags.is_empty() {
                tags.extend(filename_tags);
                filename_matched = true;
            }
        }

        // Shebang fallback
        if !filename_matched
            && info.is_executable
            && !self.skip_shebang_analysis
            && let Some(content) = info.content
            && let Ok(shebang) = parse_shebang(content)
            && let Some(interp) = shebang.first()
        {
            tags.extend(tags_from_interpreter(interp));
        }

        // Content encoding
        if !self.skip_content_analysis
            && !tags.iter().any(|t| ENCODING_TAGS.contains(t))
            && let Some(content) = info.content
            && let Ok(text) = is_text(content)
        {
            tags.insert(if text { TEXT } else { BINARY });
        }

        tags
    }

    fn analyze_filename_and_shebang_configured<P: AsRef<Path>>(
        &self,
        path: P,
        is_executable: bool,
    ) -> TagSet {
        let path = path.as_ref();
        let mut tags = TagSet::new();

        // Check filename-based tags first (including custom extensions)
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // Check custom extensions first if provided
            if let Some(custom_exts) = &self.custom_extensions
                && let Some(ext) = Path::new(filename).extension().and_then(|e| e.to_str())
                && let Some(ext_tags) = custom_exts.get(&ext.to_lowercase())
            {
                tags.extend(ext_tags.iter().copied());
                return tags; // Custom extension takes precedence
            }

            // Fall back to standard filename analysis
            let filename_tags = tags_from_filename(filename);
            if !filename_tags.is_empty() {
                tags.extend(filename_tags);
            } else if is_executable && !self.skip_shebang_analysis {
                // Parse shebang for executable files without recognized extensions
                if let Ok(shebang_components) = parse_shebang_from_file(path)
                    && let Some(interp) = shebang_components.first()
                {
                    tags.extend(tags_from_interpreter(interp));
                }
            }
        }

        tags
    }
}

/// Result type for file identification operations.
///
/// This is a convenience type alias for operations that may fail with
/// file system or parsing errors.
pub type Result<T> = std::result::Result<T, IdentifyError>;

/// Errors that can occur during file identification.
#[derive(thiserror::Error, Debug)]
pub enum IdentifyError {
    /// The specified path does not exist on the filesystem.
    #[error("{path} does not exist.")]
    PathNotFound { path: String },

    /// An I/O error occurred while accessing the file.
    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    /// The file path contains invalid UTF-8 sequences.
    #[error("Path contains invalid UTF-8: {path}")]
    InvalidPath { path: String },

    /// The file content is not valid UTF-8 when UTF-8 is expected.
    #[error("File contains invalid UTF-8 content")]
    InvalidUtf8,
}

/// Analyze file system metadata to determine basic file type.
///
/// Returns tags for directory, symlink, socket, or file based on metadata.
/// This is the first step in file identification.
fn analyze_file_type(metadata: &std::fs::Metadata) -> Option<TagSet> {
    let file_type = metadata.file_type();

    if file_type.is_dir() {
        return Some(HashSet::from([DIRECTORY]));
    }
    if file_type.is_symlink() {
        return Some(HashSet::from([SYMLINK]));
    }

    // Check for socket (Unix-specific)
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if file_type.is_socket() {
            return Some(HashSet::from([SOCKET]));
        }
    }

    // Regular file - continue with further analysis
    None
}

/// Analyze file permissions to determine executable status.
///
/// Returns true if the file is executable, false otherwise.
/// On Unix systems, checks permission bits. On other systems, checks file extension.
fn analyze_permissions<P: AsRef<Path>>(path: P, metadata: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = path; // Suppress unused warning on Unix
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        // On non-Unix systems, check file extension for common executables
        let _ = metadata; // Suppress unused warning on non-Unix
        let path = path.as_ref();
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext.to_lowercase().as_str(), "exe" | "bat" | "cmd"))
            .unwrap_or(false)
    }
}

/// Analyze filename and potentially shebang for file type identification.
///
/// First tries filename-based identification. If that fails and the file is executable,
/// falls back to shebang analysis.
fn analyze_filename_and_shebang<P: AsRef<Path>>(path: P, is_executable: bool) -> TagSet {
    let path = path.as_ref();
    let mut tags = TagSet::new();

    // Check filename-based tags first
    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
        let filename_tags = tags_from_filename(filename);
        if !filename_tags.is_empty() {
            tags.extend(filename_tags);
        } else if is_executable {
            // Parse shebang for executable files without recognized extensions
            if let Ok(shebang_components) = parse_shebang_from_file(path)
                && let Some(interp) = shebang_components.first()
            {
                tags.extend(tags_from_interpreter(interp));
            }
        }
    }

    tags
}

/// Analyze file content to determine encoding (text vs binary).
///
/// Only performs analysis if encoding tags are not already present.
fn analyze_content_encoding<P: AsRef<Path>>(path: P, existing_tags: &TagSet) -> Result<TagSet> {
    let mut tags = TagSet::new();

    // Check if we need to determine binary vs text
    if !existing_tags.iter().any(|tag| ENCODING_TAGS.contains(tag)) {
        if file_is_text(path)? {
            tags.insert(TEXT);
        } else {
            tags.insert(BINARY);
        }
    }

    Ok(tags)
}

/// Identify a file from its filesystem path.
///
/// This is the most comprehensive identification method, providing a superset
/// of information from other methods. It analyzes:
///
/// 1. File type (regular file, directory, symlink, socket)
/// 2. File permissions (executable vs non-executable)
/// 3. Filename and extension patterns
/// 4. File content (binary vs text detection)
/// 5. Shebang lines for executable files
///
/// # Arguments
///
/// * `path` - Path to the file to identify
///
/// # Returns
///
/// A set of tags identifying the file type and characteristics.
///
/// # Errors
///
/// Returns [`IdentifyError::PathNotFound`] if the path doesn't exist, or
/// [`IdentifyError::IoError`] for other I/O failures.
///
/// # Examples
///
/// ```rust
/// use file_identify::tags_from_path;
/// # use std::fs;
/// # use tempfile::tempdir;
///
/// # let dir = tempdir().unwrap();
/// # let file_path = dir.path().join("script.py");
/// # fs::write(&file_path, "#!/usr/bin/env python3\nprint('hello')").unwrap();
/// let tags = tags_from_path(&file_path).unwrap();
/// assert!(tags.contains("file"));
/// assert!(tags.contains("python"));
/// assert!(tags.contains("text"));
/// ```
pub fn tags_from_path<P: AsRef<Path>>(path: P) -> Result<TagSet> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();

    // Get file metadata
    let metadata = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(_) => {
            return Err(IdentifyError::PathNotFound {
                path: path_str.to_string(),
            });
        }
    };

    // Step 1: Check for non-regular file types (directory, symlink, socket)
    if let Some(file_type_tags) = analyze_file_type(&metadata) {
        return Ok(file_type_tags);
    }

    // Step 2: This is a regular file - start building tag set
    let mut tags = TagSet::new();
    tags.insert(FILE);

    // Step 3: Analyze permissions (executable vs non-executable)
    let is_executable = analyze_permissions(path, &metadata);
    tags.insert(if is_executable {
        EXECUTABLE
    } else {
        NON_EXECUTABLE
    });

    // Step 4: Analyze filename and potentially shebang
    tags.extend(analyze_filename_and_shebang(path, is_executable));

    // Step 5: Analyze content encoding (text vs binary) if not already determined
    tags.extend(analyze_content_encoding(path, &tags)?);

    Ok(tags)
}

/// Identify a file from pre-loaded information, without any I/O.
///
/// This is the pure equivalent of [`tags_from_path`]. The caller provides
/// all necessary file data upfront via [`FileInfo`], making it usable with
/// mocked or virtual filesystems.
///
/// # Arguments
///
/// * `info` - Pre-loaded file information
///
/// # Returns
///
/// A set of tags identifying the file type and characteristics.
///
/// # Examples
///
/// ```rust
/// use file_identify::{tags_from_info, FileInfo, FileKind};
///
/// let info = FileInfo {
///     filename: "script.py",
///     file_kind: FileKind::Regular,
///     is_executable: false,
///     content: Some(b"print('hello')"),
/// };
/// let tags = tags_from_info(&info);
/// assert!(tags.contains("file"));
/// assert!(tags.contains("python"));
/// assert!(tags.contains("text"));
///
/// // Directories return just the type tag
/// let info = FileInfo {
///     filename: "src",
///     file_kind: FileKind::Directory,
///     is_executable: false,
///     content: None,
/// };
/// let tags = tags_from_info(&info);
/// assert!(tags.contains("directory"));
/// ```
pub fn tags_from_info(info: &FileInfo<'_>) -> TagSet {
    match info.file_kind {
        FileKind::Directory => return HashSet::from([DIRECTORY]),
        FileKind::Symlink => return HashSet::from([SYMLINK]),
        FileKind::Socket => return HashSet::from([SOCKET]),
        FileKind::Regular => {}
    }

    let mut tags = TagSet::new();
    tags.insert(FILE);
    tags.insert(if info.is_executable {
        EXECUTABLE
    } else {
        NON_EXECUTABLE
    });

    // Filename/extension matching
    let filename_tags = tags_from_filename(info.filename);
    if !filename_tags.is_empty() {
        tags.extend(filename_tags);
    } else if info.is_executable {
        // Shebang fallback for executables without recognized extension
        if let Some(content) = info.content
            && let Ok(shebang) = parse_shebang(content)
            && let Some(interp) = shebang.first()
        {
            tags.extend(tags_from_interpreter(interp));
        }
    }

    // Content encoding (text vs binary) if not already determined
    if !tags.iter().any(|tag| ENCODING_TAGS.contains(tag))
        && let Some(content) = info.content
        && let Ok(text) = is_text(content)
    {
        tags.insert(if text { TEXT } else { BINARY });
    }

    tags
}

/// Identify a file based only on its filename.
///
/// This method analyzes the filename and extension to determine file type,
/// without accessing the filesystem. It's useful when you only have the
/// filename or want to avoid I/O operations.
///
/// # Arguments
///
/// * `filename` - The filename to analyze (can include path)
///
/// # Returns
///
/// A set of tags identifying the file type. Returns an empty set if
/// the filename is not recognized.
///
/// # Examples
///
/// ```rust
/// use file_identify::tags_from_filename;
///
/// let tags = tags_from_filename("script.py");
/// assert!(tags.contains("python"));
/// assert!(tags.contains("text"));
///
/// let tags = tags_from_filename("Dockerfile");
/// assert!(tags.contains("dockerfile"));
///
/// let tags = tags_from_filename("unknown.xyz");
/// assert!(tags.is_empty());
/// ```
pub fn tags_from_filename(filename: &str) -> TagSet {
    let mut tags = TagSet::new();

    // Check exact filename matches first
    for part in std::iter::once(filename).chain(filename.split('.')) {
        let name_tags = get_name_tags(part);
        if !name_tags.is_empty() {
            tags.extend(name_tags);
            break;
        }
    }

    // Check file extension
    if let Some(ext) = Path::new(filename).extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();

        let ext_tags = get_extension_tags(&ext_lower);
        if !ext_tags.is_empty() {
            tags.extend(ext_tags);
        } else {
            let binary_check_tags = get_extensions_need_binary_check_tags(&ext_lower);
            if !binary_check_tags.is_empty() {
                tags.extend(binary_check_tags);
            }
        }
    }

    tags
}

/// Identify tags based on a shebang interpreter.
///
/// This function analyzes interpreter names from shebang lines to determine
/// the script type. It handles version-specific interpreters by progressively
/// removing version suffixes.
///
/// # Arguments
///
/// * `interpreter` - The interpreter name or path from a shebang
///
/// # Returns
///
/// A set of tags for the interpreter type. Returns an empty set if
/// the interpreter is not recognized.
///
/// # Examples
///
/// ```rust
/// use file_identify::tags_from_interpreter;
///
/// let tags = tags_from_interpreter("python3.11");
/// assert!(tags.contains("python"));
/// assert!(tags.contains("python3"));
///
/// let tags = tags_from_interpreter("/usr/bin/bash");
/// assert!(tags.contains("shell"));
/// assert!(tags.contains("bash"));
///
/// let tags = tags_from_interpreter("unknown-interpreter");
/// assert!(tags.is_empty());
/// ```
pub fn tags_from_interpreter(interpreter: &str) -> TagSet {
    // Extract the interpreter name from the path
    let interpreter_name = interpreter.split('/').next_back().unwrap_or(interpreter);

    // Try progressively shorter versions (e.g., "python3.5.2" -> "python3.5" -> "python3")
    let mut current = interpreter_name;
    while !current.is_empty() {
        let tags = get_interpreter_tags(current);
        if !tags.is_empty() {
            return tags;
        }

        // Try removing the last dot-separated part
        match current.rfind('.') {
            Some(pos) => current = &current[..pos],
            None => break,
        }
    }

    TagSet::new()
}

/// Determine if a file contains text or binary data.
///
/// This function reads the first 1KB of a file to determine if it contains
/// text or binary data, using a similar algorithm to the `file` command.
///
/// # Arguments
///
/// * `path` - Path to the file to analyze
///
/// # Returns
///
/// `true` if the file appears to contain text, `false` if binary.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or read.
///
/// # Examples
///
/// ```rust
/// use file_identify::file_is_text;
/// # use std::fs;
/// # use tempfile::tempdir;
///
/// # let dir = tempdir().unwrap();
/// # let text_path = dir.path().join("text.txt");
/// # fs::write(&text_path, "Hello, world!").unwrap();
/// assert!(file_is_text(&text_path).unwrap());
///
/// # let binary_path = dir.path().join("binary.bin");
/// # fs::write(&binary_path, &[0x7f, 0x45, 0x4c, 0x46]).unwrap();
/// assert!(!file_is_text(&binary_path).unwrap());
/// ```
pub fn file_is_text<P: AsRef<Path>>(path: P) -> Result<bool> {
    let file = fs::File::open(path)?;
    is_text(file)
}

/// Determine if data from a reader contains text or binary content.
///
/// This function reads up to 1KB from the provided reader and analyzes
/// the bytes to determine if they represent text or binary data.
///
/// # Arguments
///
/// * `reader` - A reader providing the data to analyze
///
/// # Returns
///
/// `true` if the data appears to be text, `false` if binary.
///
/// # Examples
///
/// ```rust
/// use file_identify::is_text;
/// use std::io::Cursor;
///
/// let text_data = Cursor::new(b"Hello, world!");
/// assert!(is_text(text_data).unwrap());
///
/// let binary_data = Cursor::new(&[0x7f, 0x45, 0x4c, 0x46, 0x00]);
/// assert!(!is_text(binary_data).unwrap());
/// ```
pub fn is_text<R: Read>(mut reader: R) -> Result<bool> {
    // Compile-time lookup table: true for bytes that are valid in text files.
    // Covers ASCII printable (0x20..0x7F), extended ASCII (0x80..=0xFF),
    // and common control chars (BEL, BS, TAB, LF, VT, FF, CR, ESC).
    const TEXT_BYTES: [bool; 256] = {
        let mut table = [false; 256];
        let mut i = 0x20;
        while i < 0x7F {
            table[i] = true;
            i += 1;
        }
        let mut i = 0x80;
        while i < 256 {
            table[i] = true;
            i += 1;
        }
        table[7] = true;
        table[8] = true;
        table[9] = true;
        table[10] = true;
        table[11] = true;
        table[12] = true;
        table[13] = true;
        table[27] = true;
        table
    };

    let mut buffer = [0; 1024];
    let bytes_read = reader.read(&mut buffer)?;

    Ok(buffer[..bytes_read].iter().all(|&b| TEXT_BYTES[b as usize]))
}

/// Parse shebang line from an executable file and return raw shebang components.
///
/// This function reads the first line of an executable file to extract
/// shebang information and return the raw command components, similar to
/// Python's identify.parse_shebang_from_file().
///
/// # Arguments
///
/// * `path` - Path to the executable file
///
/// # Returns
///
/// A vector of raw shebang components. Returns an empty vector if:
/// - The file is not executable
/// - No shebang is found
///
/// # Errors
///
/// Returns an error if the file cannot be accessed or read.
///
/// # Examples
///
/// ```rust
/// use file_identify::parse_shebang_from_file;
/// # use std::fs;
/// # use std::os::unix::fs::PermissionsExt;
/// # use tempfile::tempdir;
///
/// # let dir = tempdir().unwrap();
/// # let script_path = dir.path().join("script");
/// # fs::write(&script_path, "#!/usr/bin/env python3\nprint('hello')").unwrap();
/// # let mut perms = fs::metadata(&script_path).unwrap().permissions();
/// # perms.set_mode(0o755);
/// # fs::set_permissions(&script_path, perms).unwrap();
/// let shebang = parse_shebang_from_file(&script_path).unwrap();
/// assert_eq!(shebang.get(0).unwrap(), "python3");
/// ```
pub fn parse_shebang_from_file<P: AsRef<Path>>(path: P) -> Result<ShebangTuple> {
    let path = path.as_ref();

    // Only check executable files
    let metadata = fs::metadata(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Ok(ShebangTuple::new());
        }
    }

    let file = fs::File::open(path)?;
    parse_shebang(file)
}

/// Parse a shebang line from a reader and return raw shebang components.
///
/// This function reads the first line from the provided reader and parses
/// it as a shebang line to extract raw command components, similar to
/// Python's identify.parse_shebang().
///
/// # Arguments
///
/// * `reader` - A reader providing the file content
///
/// # Returns
///
/// A vector of raw shebang components. Returns an empty vector if no valid shebang is found.
///
/// # Examples
///
/// ```rust
/// use file_identify::parse_shebang;
/// use std::io::Cursor;
///
/// let shebang = Cursor::new(b"#!/usr/bin/env python3\nprint('hello')");
/// let components = parse_shebang(shebang).unwrap();
/// assert_eq!(components.get(0).unwrap(), "python3");
///
/// let no_shebang = Cursor::new(b"print('hello')");
/// let components = parse_shebang(no_shebang).unwrap();
/// assert!(components.is_empty());
/// ```
pub fn parse_shebang<R: Read>(reader: R) -> Result<ShebangTuple> {
    use std::io::BufRead;

    let mut buf_reader = BufReader::new(reader);

    // Read first line efficiently using read_until
    let mut first_line_bytes = Vec::new();
    match buf_reader.read_until(b'\n', &mut first_line_bytes) {
        Ok(0) => return Ok(ShebangTuple::new()), // EOF with no data
        Ok(_) => {
            // Remove trailing newline if present
            if first_line_bytes.ends_with(b"\n") {
                first_line_bytes.pop();
            }
            // Also handle \r\n line endings
            if first_line_bytes.ends_with(b"\r") {
                first_line_bytes.pop();
            }
        }
        Err(_) => return Ok(ShebangTuple::new()), // Read error
    }

    // Check if starts with shebang
    if first_line_bytes.len() < 2 || &first_line_bytes[0..2] != b"#!" {
        return Ok(ShebangTuple::new());
    }

    // Limit line length to prevent memory issues
    if first_line_bytes.len() > 1024 {
        first_line_bytes.truncate(1024);
    }

    // Try to decode as UTF-8, return empty if invalid (like Python does)
    let first_line = match String::from_utf8(first_line_bytes) {
        Ok(line) => line,
        Err(_) => return Ok(ShebangTuple::new()),
    };

    // Remove the #! and clean up the line
    let shebang_line = first_line[2..].trim();

    // Check for only printable ASCII (like Python does)
    for c in shebang_line.chars() {
        if !c.is_ascii() || (c.is_control() && c != '\t') {
            return Ok(ShebangTuple::new());
        }
    }

    // Parse the shebang command using simple split (like Python's shlex fallback)
    let parts: smallvec::SmallVec<[&str; 4]> = shebang_line.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(ShebangTuple::new());
    }

    let cmd: smallvec::SmallVec<[&str; 2]> = if parts[0] == "/usr/bin/env" {
        if parts.len() == 1 {
            // Just "#!/usr/bin/env" with no interpreter
            smallvec::SmallVec::new()
        } else if parts.len() >= 2 && parts[1] == "-S" {
            if parts.len() > 2 {
                parts[2..].iter().copied().collect()
            } else {
                // Just "#!/usr/bin/env -S" with no interpreter
                smallvec::SmallVec::new()
            }
        } else {
            parts[1..].iter().copied().collect()
        }
    } else {
        parts.iter().copied().collect()
    };

    if cmd.is_empty() {
        return Ok(ShebangTuple::new());
    }

    // Return the raw command components as strings
    Ok(ShebangTuple::from_vec(
        cmd.iter().map(|s| s.to_string()).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::{NamedTempFile, tempdir};

    // Helper macro to create ShebangTuple from string slices for testing
    macro_rules! shebang_tuple {
        () => {
            ShebangTuple::new()
        };
        ($($item:expr),+) => {
            ShebangTuple::from_vec(vec![$($item.to_string()),+])
        };
    }

    // Test tag system completeness
    #[test]
    fn test_all_basic_tags_exist() {
        assert!(TYPE_TAGS.contains("file"));
        assert!(TYPE_TAGS.contains("directory"));
        assert!(MODE_TAGS.contains("executable"));
        assert!(ENCODING_TAGS.contains("text"));
    }

    #[test]
    fn test_tag_groups_are_disjoint() {
        assert!(TYPE_TAGS.is_disjoint(&MODE_TAGS));
        assert!(TYPE_TAGS.is_disjoint(&ENCODING_TAGS));
        assert!(MODE_TAGS.is_disjoint(&ENCODING_TAGS));
    }

    // Test tags_from_filename with various scenarios
    #[test]
    fn test_tags_from_filename_basic() {
        let tags = tags_from_filename("file.py");
        assert!(tags.contains("text"));
        assert!(tags.contains("python"));
    }

    #[test]
    fn test_tags_from_filename_special_names() {
        let tags = tags_from_filename("Dockerfile");
        assert!(tags.contains("dockerfile"));
        assert!(tags.contains("text"));

        let tags = tags_from_filename("Makefile");
        assert!(tags.contains("makefile"));
        assert!(tags.contains("text"));

        let tags = tags_from_filename("Cargo.toml");
        assert!(tags.contains("toml"));
        assert!(tags.contains("cargo"));
    }

    #[test]
    fn test_tags_from_filename_case_insensitive_extension() {
        let tags = tags_from_filename("image.JPG");
        assert!(tags.contains("binary"));
        assert!(tags.contains("image"));
        assert!(tags.contains("jpeg"));
    }

    #[test]
    fn test_tags_from_filename_precedence() {
        // setup.cfg should match by name, not .cfg extension
        let tags = tags_from_filename("setup.cfg");
        assert!(tags.contains("ini"));
    }

    #[test]
    fn test_tags_from_filename_complex_names() {
        let tags = tags_from_filename("Dockerfile.xenial");
        assert!(tags.contains("dockerfile"));

        let tags = tags_from_filename("README.md");
        assert!(tags.contains("markdown"));
        assert!(tags.contains("plain-text"));
    }

    #[test]
    fn test_tags_from_filename_unrecognized() {
        let tags = tags_from_filename("unknown.xyz");
        assert!(tags.is_empty());

        let tags = tags_from_filename("noextension");
        assert!(tags.is_empty());
    }

    // Test tags_from_interpreter
    #[test]
    fn test_tags_from_interpreter_basic() {
        let tags = tags_from_interpreter("python3");
        assert!(tags.contains("python"));
        assert!(tags.contains("python3"));
    }

    #[test]
    fn test_tags_from_interpreter_versioned() {
        let tags = tags_from_interpreter("python3.11.2");
        assert!(tags.contains("python"));
        assert!(tags.contains("python3"));

        let tags = tags_from_interpreter("php8.1");
        assert!(tags.contains("php"));
        assert!(tags.contains("php8"));
    }

    #[test]
    fn test_tags_from_interpreter_with_path() {
        let tags = tags_from_interpreter("/usr/bin/python3");
        assert!(tags.contains("python"));
        assert!(tags.contains("python3"));
    }

    #[test]
    fn test_tags_from_interpreter_unrecognized() {
        let tags = tags_from_interpreter("unknown-interpreter");
        assert!(tags.is_empty());

        let tags = tags_from_interpreter("");
        assert!(tags.is_empty());
    }

    // Test is_text function
    #[test]
    fn test_is_text_basic() {
        assert!(is_text(Cursor::new(b"hello world")).unwrap());
        assert!(is_text(Cursor::new(b"")).unwrap());
        assert!(!is_text(Cursor::new(b"hello\x00world")).unwrap());
    }

    #[test]
    fn test_is_text_unicode() {
        assert!(is_text(Cursor::new("éóñəå  ⊂(◉‿◉)つ(ノ≥∇≤)ノ".as_bytes())).unwrap());
        assert!(is_text(Cursor::new(r"¯\_(ツ)_/¯".as_bytes())).unwrap());
        assert!(is_text(Cursor::new("♪┏(・o･)┛♪┗ ( ･o･) ┓♪".as_bytes())).unwrap());
    }

    #[test]
    fn test_is_text_binary_data() {
        // ELF header
        assert!(!is_text(Cursor::new(&[0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01])).unwrap());
        // Random binary data
        assert!(!is_text(Cursor::new(&[0x43, 0x92, 0xd9, 0x0f, 0xaf, 0x32, 0x2c])).unwrap());
    }

    // Test parse_shebang function
    #[test]
    fn test_parse_shebang_basic() {
        let components = parse_shebang(Cursor::new(b"#!/usr/bin/python")).unwrap();
        assert_eq!(components, shebang_tuple!["/usr/bin/python"]);

        let components = parse_shebang(Cursor::new(b"#!/usr/bin/env python")).unwrap();
        assert_eq!(components, shebang_tuple!["python"]);
    }

    #[test]
    fn test_parse_shebang_env_with_flags() {
        let components = parse_shebang(Cursor::new(b"#!/usr/bin/env -S python -u")).unwrap();
        assert_eq!(components, shebang_tuple!["python", "-u"]);
    }

    #[test]
    fn test_parse_shebang_spaces() {
        let components = parse_shebang(Cursor::new(b"#! /usr/bin/python")).unwrap();
        assert_eq!(components, shebang_tuple!["/usr/bin/python"]);

        let components = parse_shebang(Cursor::new(b"#!/usr/bin/foo  python")).unwrap();
        assert_eq!(components, shebang_tuple!["/usr/bin/foo", "python"]);
    }

    #[test]
    fn test_parse_shebang_no_shebang() {
        let components = parse_shebang(Cursor::new(b"import sys")).unwrap();
        assert!(components.is_empty());

        let components = parse_shebang(Cursor::new(b"")).unwrap();
        assert!(components.is_empty());
    }

    #[test]
    fn test_parse_shebang_invalid_utf8() {
        let result = parse_shebang(Cursor::new(&[0x23, 0x21, 0xf9, 0x93, 0x01, 0x42, 0xcd]));
        match result {
            Ok(components) => assert!(components.is_empty()),
            Err(_) => (), // I/O errors are acceptable for invalid UTF-8 data
        }
    }

    // File system tests using tempfiles
    #[test]
    fn test_tags_from_path_file_not_found() {
        let result = tags_from_path("/nonexistent/path");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_tags_from_path_regular_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(&file, "print('hello')").unwrap();

        let tags = tags_from_path(file.path()).unwrap();
        assert!(tags.contains("file"));
        assert!(tags.contains("non-executable"));
        assert!(tags.contains("text"));
    }

    #[test]
    fn test_tags_from_path_executable_file() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("script.py");
        fs::write(&script_path, "#!/usr/bin/env python3\nprint('hello')").unwrap();

        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        let tags = tags_from_path(&script_path).unwrap();
        assert!(tags.contains("file"));
        assert!(tags.contains("executable"));
        assert!(tags.contains("python"));
        assert!(tags.contains("text"));
    }

    #[test]
    fn test_tags_from_path_directory() {
        let dir = tempdir().unwrap();
        let tags = tags_from_path(dir.path()).unwrap();
        assert_eq!(tags, HashSet::from(["directory"]));
    }

    #[test]
    fn test_tags_from_path_binary_file() {
        let dir = tempdir().unwrap();
        let binary_path = dir.path().join("binary");
        fs::write(&binary_path, &[0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01]).unwrap();

        let tags = tags_from_path(&binary_path).unwrap();
        assert!(tags.contains("file"));
        assert!(tags.contains("binary"));
        assert!(tags.contains("non-executable"));
    }

    #[test]
    fn test_file_is_text_simple() {
        let dir = tempdir().unwrap();
        let text_path = dir.path().join("text.txt");
        fs::write(&text_path, "Hello, world!").unwrap();
        assert!(file_is_text(&text_path).unwrap());
    }

    #[test]
    fn test_file_is_text_does_not_exist() {
        let result = file_is_text("/nonexistent/file");
        assert!(result.is_err());
    }

    // Test extensions that need binary check
    #[test]
    fn test_plist_binary_detection() {
        let dir = tempdir().unwrap();
        let plist_path = dir.path().join("test.plist");

        // Binary plist
        let binary_plist = [
            0x62, 0x70, 0x6c, 0x69, 0x73, 0x74, 0x30, 0x30, // "bplist00"
            0xd1, 0x01, 0x02, 0x5f, 0x10, 0x0f,
        ];
        fs::write(&plist_path, &binary_plist).unwrap();

        let tags = tags_from_path(&plist_path).unwrap();
        assert!(tags.contains("plist"));
        assert!(tags.contains("binary"));
    }

    #[test]
    fn test_plist_text_detection() {
        let dir = tempdir().unwrap();
        let plist_path = dir.path().join("test.plist");

        let text_plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>TestKey</key>
    <string>TestValue</string>
</dict>
</plist>"#;
        fs::write(&plist_path, text_plist).unwrap();

        let tags = tags_from_path(&plist_path).unwrap();
        assert!(tags.contains("plist"));
        assert!(tags.contains("text"));
    }

    // Additional edge case tests
    #[test]
    fn test_empty_file() {
        let dir = tempdir().unwrap();
        let empty_path = dir.path().join("empty");
        fs::write(&empty_path, "").unwrap();

        let tags = tags_from_path(&empty_path).unwrap();
        assert!(tags.contains("file"));
        assert!(tags.contains("text")); // Empty files are considered text
        assert!(tags.contains("non-executable"));
    }

    #[test]
    fn test_shebang_incomplete() {
        let shebang_incomplete = parse_shebang(Cursor::new(b"#!   \n")).unwrap();
        assert!(shebang_incomplete.is_empty());
    }

    #[test]
    fn test_multiple_extensions() {
        let tags = tags_from_filename("backup.tar.gz");
        assert!(tags.contains("binary"));
        assert!(tags.contains("gzip"));
    }

    // Test FileIdentifier builder pattern
    #[test]
    fn test_file_identifier_default() {
        let dir = tempdir().unwrap();
        let py_file = dir.path().join("test.py");
        fs::write(&py_file, "print('hello')").unwrap();

        let identifier = FileIdentifier::new();
        let tags = identifier.identify(&py_file).unwrap();

        assert!(tags.contains("file"));
        assert!(tags.contains("python"));
        assert!(tags.contains("text"));
        assert!(tags.contains("non-executable"));
    }

    #[test]
    fn test_file_identifier_skip_content_analysis() {
        let dir = tempdir().unwrap();
        let unknown_file = dir.path().join("unknown_file");
        fs::write(&unknown_file, "some content").unwrap();

        let identifier = FileIdentifier::new().skip_content_analysis();
        let tags = identifier.identify(&unknown_file).unwrap();

        assert!(tags.contains("file"));
        assert!(tags.contains("non-executable"));
        // Should not have text or binary tags since content analysis was skipped
        assert!(!tags.contains("text"));
        assert!(!tags.contains("binary"));
    }

    #[test]
    fn test_file_identifier_skip_shebang_analysis() {
        let dir = tempdir().unwrap();
        let script_file = dir.path().join("script");
        fs::write(&script_file, "#!/usr/bin/env python3\nprint('hello')").unwrap();

        let mut perms = fs::metadata(&script_file).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_file, perms).unwrap();

        let identifier = FileIdentifier::new().skip_shebang_analysis();
        let tags = identifier.identify(&script_file).unwrap();

        assert!(tags.contains("file"));
        assert!(tags.contains("executable"));
        // Should not have python tags since shebang analysis was skipped
        // and filename doesn't match any patterns
        assert!(!tags.contains("python"));
    }

    #[test]
    fn test_file_identifier_custom_extensions() {
        let dir = tempdir().unwrap();
        let custom_file = dir.path().join("test.myext");
        fs::write(&custom_file, "custom content").unwrap();

        let mut custom_extensions = std::collections::HashMap::new();
        custom_extensions.insert("myext".to_string(), HashSet::from(["custom", "text"]));

        let identifier = FileIdentifier::new().with_custom_extensions(custom_extensions);
        let tags = identifier.identify(&custom_file).unwrap();

        assert!(tags.contains("file"));
        assert!(tags.contains("custom"));
        assert!(tags.contains("text"));
        assert!(tags.contains("non-executable"));
    }

    #[test]
    fn test_file_identifier_chaining() {
        let dir = tempdir().unwrap();
        let test_file = dir.path().join("test.unknown");
        fs::write(&test_file, "content").unwrap();

        let identifier = FileIdentifier::new()
            .skip_content_analysis()
            .skip_shebang_analysis();
        let tags = identifier.identify(&test_file).unwrap();

        assert!(tags.contains("file"));
        assert!(tags.contains("non-executable"));
        // Should have minimal tags due to skipping analyses
        assert!(!tags.contains("text"));
        assert!(!tags.contains("binary"));
    }

    // Additional comprehensive tests from Python version
    #[test]
    fn test_comprehensive_shebang_parsing() {
        let test_cases = vec![
            ("", vec![]),
            ("#!/usr/bin/python", vec!["python"]),
            ("#!/usr/bin/env python", vec!["python"]),
            ("#! /usr/bin/python", vec!["python"]),
            ("#!/usr/bin/foo  python", vec![]), // "foo" not recognized
            ("#!/usr/bin/env -S python -u", vec!["python"]),
            ("#!/usr/bin/env", vec![]),
            ("#!/usr/bin/env -S", vec![]),
        ];

        for (input, _expected) in test_cases {
            let components = parse_shebang(Cursor::new(input.as_bytes())).unwrap();

            match input {
                "" => assert!(components.is_empty()),
                "#!/usr/bin/python" => assert_eq!(components, shebang_tuple!["/usr/bin/python"]),
                "#!/usr/bin/env python" => assert_eq!(components, shebang_tuple!["python"]),
                "#! /usr/bin/python" => assert_eq!(components, shebang_tuple!["/usr/bin/python"]),
                "#!/usr/bin/foo  python" => {
                    assert_eq!(components, shebang_tuple!["/usr/bin/foo", "python"])
                }
                "#!/usr/bin/env -S python -u" => {
                    assert_eq!(components, shebang_tuple!["python", "-u"])
                }
                "#!/usr/bin/env" => {
                    // This should be empty since no interpreter specified
                    assert!(
                        components.is_empty(),
                        "Got components: {:?} for input: '{}'",
                        components,
                        input
                    );
                }
                "#!/usr/bin/env -S" => {
                    // This should be empty since no interpreter after -S
                    assert!(
                        components.is_empty(),
                        "Got components: {:?} for input: '{}'",
                        components,
                        input
                    );
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_invalid_utf8_shebang() {
        // Test that invalid UTF-8 in shebang doesn't crash
        let invalid_utf8_cases = vec![
            &[0xf9, 0x93, 0x01, 0x42, 0xcd][..],
            &[0x23, 0x21, 0xf9, 0x93, 0x01, 0x42, 0xcd][..],
            &[0x23, 0x21, 0x00, 0x00, 0x00, 0x00][..],
        ];

        for input in invalid_utf8_cases {
            // Should not panic, should return empty components for invalid UTF-8
            let result = parse_shebang(Cursor::new(input));
            match result {
                Ok(components) => assert!(components.is_empty()),
                Err(_) => (), // I/O errors are acceptable for invalid data
            }
        }
    }

    // Tests for tags_from_info (I/O-free API)

    #[test]
    fn test_tags_from_info_regular_file() {
        let info = FileInfo {
            filename: "script.py",
            file_kind: FileKind::Regular,
            is_executable: false,
            content: Some(b"print('hello')"),
        };
        let tags = tags_from_info(&info);
        assert!(tags.contains("file"));
        assert!(tags.contains("non-executable"));
        assert!(tags.contains("python"));
        assert!(tags.contains("text"));
    }

    #[test]
    fn test_tags_from_info_directory() {
        let info = FileInfo {
            filename: "src",
            file_kind: FileKind::Directory,
            is_executable: false,
            content: None,
        };
        let tags = tags_from_info(&info);
        assert!(tags.contains("directory"));
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn test_tags_from_info_symlink() {
        let info = FileInfo {
            filename: "link",
            file_kind: FileKind::Symlink,
            is_executable: false,
            content: None,
        };
        let tags = tags_from_info(&info);
        assert!(tags.contains("symlink"));
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn test_tags_from_info_socket() {
        let info = FileInfo {
            filename: "sock",
            file_kind: FileKind::Socket,
            is_executable: false,
            content: None,
        };
        let tags = tags_from_info(&info);
        assert!(tags.contains("socket"));
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn test_tags_from_info_executable_with_shebang() {
        let info = FileInfo {
            filename: "my-script",
            file_kind: FileKind::Regular,
            is_executable: true,
            content: Some(b"#!/usr/bin/env python3\nprint('hello')"),
        };
        let tags = tags_from_info(&info);
        assert!(tags.contains("file"));
        assert!(tags.contains("executable"));
        assert!(tags.contains("python"));
        assert!(tags.contains("python3"));
        assert!(tags.contains("text"));
    }

    #[test]
    fn test_tags_from_info_binary_content() {
        let info = FileInfo {
            filename: "data.bin",
            file_kind: FileKind::Regular,
            is_executable: false,
            content: Some(&[0x7f, 0x45, 0x4c, 0x46, 0x00]),
        };
        let tags = tags_from_info(&info);
        assert!(tags.contains("file"));
        assert!(tags.contains("binary"));
    }

    #[test]
    fn test_tags_from_info_no_content() {
        let info = FileInfo {
            filename: "unknown",
            file_kind: FileKind::Regular,
            is_executable: false,
            content: None,
        };
        let tags = tags_from_info(&info);
        assert!(tags.contains("file"));
        assert!(tags.contains("non-executable"));
        // No encoding tag since no content was provided
        assert!(!tags.contains("text"));
        assert!(!tags.contains("binary"));
    }

    #[test]
    fn test_tags_from_info_extension_provides_encoding() {
        let info = FileInfo {
            filename: "app.js",
            file_kind: FileKind::Regular,
            is_executable: false,
            content: None,
        };
        let tags = tags_from_info(&info);
        assert!(tags.contains("javascript"));
        assert!(tags.contains("text"));
    }

    #[test]
    fn test_identify_from_with_custom_extensions() {
        let mut custom = std::collections::HashMap::new();
        custom.insert("myext".to_string(), HashSet::from(["text", "custom-lang"]));

        let identifier = FileIdentifier::new().with_custom_extensions(custom);
        let info = FileInfo {
            filename: "code.myext",
            file_kind: FileKind::Regular,
            is_executable: false,
            content: Some(b"some code"),
        };
        let tags = identifier.identify_from(&info);
        assert!(tags.contains("custom-lang"));
        assert!(tags.contains("text"));
    }

    #[test]
    fn test_identify_from_skip_content() {
        let identifier = FileIdentifier::new().skip_content_analysis();
        let info = FileInfo {
            filename: "unknown",
            file_kind: FileKind::Regular,
            is_executable: false,
            content: Some(b"hello world"),
        };
        let tags = identifier.identify_from(&info);
        assert!(!tags.contains("text"));
        assert!(!tags.contains("binary"));
    }

    #[test]
    fn test_identify_from_skip_shebang() {
        let identifier = FileIdentifier::new().skip_shebang_analysis();
        let info = FileInfo {
            filename: "my-script",
            file_kind: FileKind::Regular,
            is_executable: true,
            content: Some(b"#!/usr/bin/env python3\nprint('hello')"),
        };
        let tags = identifier.identify_from(&info);
        assert!(!tags.contains("python"));
        // Still detects text encoding
        assert!(tags.contains("text"));
    }
}
