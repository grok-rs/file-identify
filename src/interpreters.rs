use crate::tags::{TagSet, tags_from_array};
use phf::phf_map;

// Interpreter mappings using Perfect Hash Functions for compile-time optimization.

pub static INTERPRETER_TAGS: phf::Map<&'static str, &'static [&'static str]> = phf_map! {
    "ash" => &["shell", "ash"],
    "awk" => &["awk"],
    "bash" => &["shell", "bash"],
    "bats" => &["shell", "bash", "bats"],
    "cbsd" => &["shell", "cbsd"],
    "csh" => &["shell", "csh"],
    "dash" => &["shell", "dash"],
    "expect" => &["expect"],
    "ksh" => &["shell", "ksh"],
    "node" => &["javascript"],
    "nodejs" => &["javascript"],
    "perl" => &["perl"],
    "php" => &["php"],
    "php7" => &["php", "php7"],
    "php8" => &["php", "php8"],
    "python" => &["python"],
    "python2" => &["python", "python2"],
    "python3" => &["python", "python3"],
    "ruby" => &["ruby"],
    "sh" => &["shell", "sh"],
    "tcsh" => &["shell", "tcsh"],
    "zsh" => &["shell", "zsh"],
};

/// Get tags for a given interpreter using compile-time optimized lookup.
pub fn get_interpreter_tags(interpreter: &str) -> TagSet {
    INTERPRETER_TAGS
        .get(interpreter)
        .map(|&tags| tags_from_array(tags))
        .unwrap_or_default()
}
