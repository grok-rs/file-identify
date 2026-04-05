use std::collections::HashSet;
use std::sync::LazyLock;

use crate::extensions;
use crate::interpreters;

pub const DIRECTORY: &str = "directory";
pub const SYMLINK: &str = "symlink";
pub const SOCKET: &str = "socket";
pub const FILE: &str = "file";
pub const EXECUTABLE: &str = "executable";
pub const NON_EXECUTABLE: &str = "non-executable";
pub const TEXT: &str = "text";
pub const BINARY: &str = "binary";

pub type TagSet = HashSet<&'static str>;

/// Helper function to convert a static array of tags to a TagSet.
#[inline]
pub fn tags_from_array(tags: &[&'static str]) -> TagSet {
    tags.iter().copied().collect()
}

pub static TYPE_TAGS: LazyLock<TagSet> =
    LazyLock::new(|| HashSet::from([DIRECTORY, FILE, SYMLINK, SOCKET]));
pub static MODE_TAGS: LazyLock<TagSet> =
    LazyLock::new(|| HashSet::from([EXECUTABLE, NON_EXECUTABLE]));
pub static ENCODING_TAGS: LazyLock<TagSet> = LazyLock::new(|| HashSet::from([BINARY, TEXT]));

/// Check if a tag is a file type tag (optimized with pattern matching)
pub fn is_type_tag(tag: &str) -> bool {
    matches!(tag, DIRECTORY | FILE | SYMLINK | SOCKET)
}

/// Check if a tag is a file mode tag (optimized with pattern matching)  
pub fn is_mode_tag(tag: &str) -> bool {
    matches!(tag, EXECUTABLE | NON_EXECUTABLE)
}

/// Check if a tag is an encoding tag (optimized with pattern matching)
pub fn is_encoding_tag(tag: &str) -> bool {
    matches!(tag, BINARY | TEXT)
}

/// `ALL_TAGS = frozenset(_ALL_TAGS)`
///
/// The set of every recognized tag — built from type, mode, encoding tags
/// plus every tag from extensions, names, and interpreters.
///
/// Mirrors Python's `identify.identify.ALL_TAGS`.
pub static ALL_TAGS: LazyLock<TagSet> = LazyLock::new(|| {
    let mut tags = TagSet::new();

    // _ALL_TAGS = {*TYPE_TAGS, *MODE_TAGS, *ENCODING_TAGS}
    tags.extend(TYPE_TAGS.iter());
    tags.extend(MODE_TAGS.iter());
    tags.extend(ENCODING_TAGS.iter());

    // _ALL_TAGS.update(*extensions.EXTENSIONS.values())
    for entry in extensions::EXTENSION_TAGS.values() {
        tags.extend(entry.iter().copied());
    }

    // _ALL_TAGS.update(*extensions.EXTENSIONS_NEED_BINARY_CHECK.values())
    for entry in extensions::EXTENSIONS_NEED_BINARY_CHECK_TAGS.values() {
        tags.extend(entry.iter().copied());
    }

    // _ALL_TAGS.update(*extensions.NAMES.values())
    for entry in extensions::NAME_TAGS.values() {
        tags.extend(entry.iter().copied());
    }

    // _ALL_TAGS.update(*interpreters.INTERPRETERS.values())
    for entry in interpreters::INTERPRETER_TAGS.values() {
        tags.extend(entry.iter().copied());
    }

    tags
});

/// The kind of filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Regular,
    Directory,
    Symlink,
    Socket,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_tags_contains_basics() {
        // type tags
        assert!(ALL_TAGS.contains("file"));
        assert!(ALL_TAGS.contains("directory"));
        // mode tags
        assert!(ALL_TAGS.contains("executable"));
        assert!(ALL_TAGS.contains("non-executable"));
        // encoding tags
        assert!(ALL_TAGS.contains("text"));
        assert!(ALL_TAGS.contains("binary"));
        // language tags from extensions
        assert!(ALL_TAGS.contains("python"));
        assert!(ALL_TAGS.contains("rust"));
        assert!(ALL_TAGS.contains("javascript"));
        assert!(ALL_TAGS.contains("json"));
        // tags from interpreters
        assert!(ALL_TAGS.contains("shell"));
        assert!(ALL_TAGS.contains("ruby"));
        assert!(ALL_TAGS.contains("perl"));
    }

    #[test]
    fn test_all_tags_does_not_contain_garbage() {
        assert!(!ALL_TAGS.contains("notarealtag"));
    }
}
