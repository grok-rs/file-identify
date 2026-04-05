use std::collections::HashSet;
use std::sync::LazyLock;

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

/// The kind of filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Regular,
    Directory,
    Symlink,
    Socket,
}
