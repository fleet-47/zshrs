//! Only the two explicitly authorized dependency-API test migrations may
//! touch upstream tests; other overlays preserve their source.

use std::fs;
use std::path::Path;

fn patch_path<'a>(header: &'a str, prefix: &str, patch: &Path) -> Option<&'a str> {
    let path = header.split('\t').next().expect("patch header path");
    if path == "/dev/null" {
        return None;
    }
    let path = path.strip_prefix(prefix).expect("expected a/ or b/ header");
    assert!(
        path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.contains('\\')
                && !part.chars().any(char::is_whitespace)
        }),
        "patch path must be a plain relative path: {path}"
    );
    let authorized_api_migration = (patch.ends_with("upgrades/rand.patch")
        && path == "tests/startup_file_parity.rs")
        || (patch.ends_with("upgrades/ureq.patch") && path == "tests/daemon_http.rs");
    assert!(
        !matches!(path.split('/').next(), Some("test" | "tests")) || authorized_api_migration,
        "test path outside the authorized API migrations: {path}"
    );
    Some(path)
}

fn hunk_count(range: &str, prefix: char) -> usize {
    let range = range
        .strip_prefix(prefix)
        .expect("expected hunk range sign");
    let (start, count) = range.split_once(',').unwrap_or((range, "1"));
    start.parse::<usize>().expect("numeric hunk start");
    count.parse().expect("numeric hunk length")
}

fn check_patch(path: &Path) {
    let contents = fs::read_to_string(path).expect("read active patch");
    let mut lines = contents.lines().peekable();
    let mut files = 0;
    while let Some(line) = lines.next() {
        let old = line.strip_prefix("--- ").expect("expected old-file header");
        let new = lines
            .next()
            .and_then(|line| line.strip_prefix("+++ "))
            .expect("expected paired new-file header");
        let old = patch_path(old, "a/", path);
        let new = patch_path(new, "b/", path);
        assert!(old.is_some() || new.is_some(), "both paths are /dev/null");
        let mut hunks = 0;
        while lines.peek().is_some_and(|line| line.starts_with("@@ ")) {
            let mut header = lines.next().expect("hunk header").split_whitespace();
            assert_eq!(header.next(), Some("@@"));
            let mut removed = hunk_count(header.next().expect("old range"), '-');
            let mut added = hunk_count(header.next().expect("new range"), '+');
            assert_eq!(header.next(), Some("@@"));
            assert!(old.is_some() || removed == 0, "new file removes content");
            assert!(new.is_some() || added == 0, "deleted file adds content");
            while removed != 0 || added != 0 {
                let line = lines.next().expect("complete hunk body");
                let (old_line, new_line) = match line.as_bytes().first() {
                    Some(b' ') => (1, 1),
                    Some(b'-') => (1, 0),
                    Some(b'+') => (0, 1),
                    _ if line == "\\ No newline at end of file" => continue,
                    _ => panic!("invalid hunk body in {}", path.display()),
                };
                removed = removed.checked_sub(old_line).expect("old hunk length");
                added = added.checked_sub(new_line).expect("new hunk length");
            }
            if lines.peek() == Some(&"\\ No newline at end of file") {
                lines.next();
            }
            hunks += 1;
        }
        assert!(hunks > 0, "file header needs at least one hunk");
        files += 1;
    }
    assert!(files > 0, "empty patch: {}", path.display());
}

#[test]
fn active_overlays_only_touch_authorized_test_paths() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("zshrs-android/patches");
    let mut patches = 0;
    for group in ["android", "upgrades"] {
        for entry in fs::read_dir(root.join(group)).expect("active patch group") {
            let entry = entry.expect("patch directory entry");
            assert!(entry.file_type().expect("patch file type").is_file());
            let path = entry.path();
            assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("patch"));
            check_patch(&path);
            patches += 1;
        }
    }
    assert!(patches > 0, "active overlay contains no patches");
}
