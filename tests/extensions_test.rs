use file_identify::extensions::{EXTENSION_TAGS, EXTENSIONS_NEED_BINARY_CHECK_TAGS, NAME_TAGS};
use file_identify::tags::tags_from_array;
use std::collections::HashSet;

#[test]
fn test_extensions_have_binary_or_text() {
    for (extension, &tag_array) in EXTENSION_TAGS.entries() {
        let tags = tags_from_array(tag_array);
        let text_binary_tags: HashSet<&str> = HashSet::from(["text", "binary"]);
        let intersection: HashSet<_> = tags.intersection(&text_binary_tags).collect();
        assert_eq!(
            intersection.len(),
            1,
            "Extension '{}' should have exactly one of 'text' or 'binary', got: {:?}",
            extension,
            tags
        );
    }
}

#[test]
fn test_names_have_binary_or_text() {
    for (name, &tag_array) in NAME_TAGS.entries() {
        let tags = tags_from_array(tag_array);
        let text_binary_tags: HashSet<&str> = HashSet::from(["text", "binary"]);
        let intersection: HashSet<_> = tags.intersection(&text_binary_tags).collect();
        assert_eq!(
            intersection.len(),
            1,
            "Name '{}' should have exactly one of 'text' or 'binary', got: {:?}",
            name,
            tags
        );
    }
}

#[test]
fn test_need_binary_check_do_not_specify_text_binary() {
    for (extension, &tag_array) in EXTENSIONS_NEED_BINARY_CHECK_TAGS.entries() {
        let tags = tags_from_array(tag_array);
        let text_binary_tags: HashSet<&str> = HashSet::from(["text", "binary"]);
        let intersection: HashSet<_> = tags.intersection(&text_binary_tags).collect();
        assert_eq!(
            intersection.len(),
            0,
            "Extension '{}' in EXTENSIONS_NEED_BINARY_CHECK should not specify 'text' or 'binary', got: {:?}",
            extension,
            tags
        );
    }
}

#[test]
fn test_mutually_exclusive_check_types() {
    let extensions_keys: HashSet<_> = EXTENSION_TAGS.keys().collect();
    let need_binary_keys: HashSet<_> = EXTENSIONS_NEED_BINARY_CHECK_TAGS.keys().collect();

    let intersection: HashSet<_> = extensions_keys.intersection(&need_binary_keys).collect();
    assert!(
        intersection.is_empty(),
        "EXTENSION_TAGS and EXTENSIONS_NEED_BINARY_CHECK_TAGS should be mutually exclusive, found overlap: {:?}",
        intersection
    );
}
