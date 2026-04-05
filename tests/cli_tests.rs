use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use tempfile::tempdir;

fn get_cli_path() -> std::path::PathBuf {
    let mut path = std::env::current_dir().unwrap();
    path.push("target");
    path.push("debug");
    path.push("file-identify");
    path
}

/// Skip CLI tests when the binary isn't built (requires --features cli).
macro_rules! require_cli_binary {
    () => {
        if !get_cli_path().exists() {
            eprintln!("Skipping: binary not built (build with --features cli)");
            return;
        }
    };
}

#[test]
fn test_cli_basic_usage() {
    require_cli_binary!();
    let dir = tempdir().unwrap();
    let py_path = dir.path().join("test.py");
    fs::write(&py_path, "print('hello')").unwrap();

    let output = Command::new(get_cli_path())
        .arg(py_path.to_str().unwrap())
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should be JSON array
    let tags: Vec<String> = serde_json::from_str(&stdout.trim()).unwrap();
    assert!(tags.contains(&"file".to_string()));
    assert!(tags.contains(&"python".to_string()));
    assert!(tags.contains(&"text".to_string()));
    assert!(tags.contains(&"non-executable".to_string()));
}

#[test]
fn test_cli_filename_only() {
    require_cli_binary!();
    let output = Command::new(get_cli_path())
        .args(&["--filename-only", "test.py"])
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    let tags: Vec<String> = serde_json::from_str(&stdout.trim()).unwrap();
    assert!(tags.contains(&"python".to_string()));
    assert!(tags.contains(&"text".to_string()));
    // Should not contain file system tags
    assert!(!tags.contains(&"file".to_string()));
}

#[test]
fn test_cli_file_not_found() {
    require_cli_binary!();
    let output = Command::new(get_cli_path())
        .arg("/nonexistent/file")
        .output()
        .expect("Failed to execute CLI");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("does not exist"));
}

#[test]
fn test_cli_unrecognized_file() {
    require_cli_binary!();
    let output = Command::new(get_cli_path())
        .args(&["--filename-only", "unknown.xyz"])
        .output()
        .expect("Failed to execute CLI");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.trim().is_empty());
}

#[test]
fn test_cli_help() {
    require_cli_binary!();
    let output = Command::new(get_cli_path())
        .arg("--help")
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("File identification tool"));
    assert!(stdout.contains("--filename-only"));
}

#[test]
fn test_cli_version() {
    require_cli_binary!();
    let output = Command::new(get_cli_path())
        .arg("--version")
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("file-identify"));
}

#[test]
fn test_cli_directory() {
    require_cli_binary!();
    let dir = tempdir().unwrap();

    let output = Command::new(get_cli_path())
        .arg(dir.path().to_str().unwrap())
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    let tags: Vec<String> = serde_json::from_str(&stdout.trim()).unwrap();
    assert_eq!(tags, vec!["directory"]);
}

#[test]
fn test_cli_executable_script() {
    require_cli_binary!();
    let dir = tempdir().unwrap();
    let script_path = dir.path().join("script");
    fs::write(&script_path, "#!/bin/bash\necho hello").unwrap();

    let mut perms = fs::metadata(&script_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).unwrap();

    let output = Command::new(get_cli_path())
        .arg(script_path.to_str().unwrap())
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    let tags: Vec<String> = serde_json::from_str(&stdout.trim()).unwrap();
    assert!(tags.contains(&"file".to_string()));
    assert!(tags.contains(&"executable".to_string()));
    assert!(tags.contains(&"shell".to_string()));
    assert!(tags.contains(&"bash".to_string()));
}

#[test]
fn test_cli_binary_file() {
    require_cli_binary!();
    let dir = tempdir().unwrap();
    let binary_path = dir.path().join("binary.exe");
    // ELF header
    fs::write(&binary_path, &[0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01]).unwrap();

    let output = Command::new(get_cli_path())
        .arg(binary_path.to_str().unwrap())
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();

    let tags: Vec<String> = serde_json::from_str(&stdout.trim()).unwrap();
    assert!(tags.contains(&"file".to_string()));
    assert!(tags.contains(&"binary".to_string()));
    assert!(tags.contains(&"non-executable".to_string()));
}
