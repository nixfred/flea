use crate::backend::listing::Listing;
use crate::error::{from_io, FleaError};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::time::Instant;

// Phase 1 never stats: d_type is free, see AGENTS.md "Two-phase listing".
// hidden:false is the shell's own dotfile convention, matched here rather than left to the client.
pub fn scan(path: &str, hidden: bool) -> Result<(Listing, f64), FleaError> {
    let t = Instant::now();
    let rd = match fs::read_dir(path) {
        Ok(rd) => rd,
        Err(e) => return Err(from_io("scan", path, &e)),
    };
    let mut l = Listing::new();
    // corner: unreadable entries skip, typeless ones become files, see AGENTS.md.
    // corner: a non-UTF8 name goes lossy here and then cannot be stat'd, see AGENTS.md.
    for entry in rd.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // corner: a dot-prefixed name is dropped before any stat, so a hidden directory costs nothing when hidden is false.
        if !hidden && name.starts_with('.') {
            continue;
        }
        let is_dir = entry.file_type().map(|f| f.is_dir()).unwrap_or(false);
        l.push(&name, is_dir);
    }
    Ok((l, t.elapsed().as_secs_f64() * 1000.0))
}

// The st_mode of a path whose listing failed. The stat outlives the denial: /root answers mode
// 0o40750 to anyone while opendir on it is refused, which is what lets a denied pane draw the
// directory's own permission string instead of nothing. Zero when the stat failed too, and a real
// st_mode always carries its file-type bits, so zero can only mean "I could not look".
pub fn mode_of(path: &str) -> u32 {
    match fs::metadata(path) {
        Ok(m) => m.mode(),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn temp_dir(tag: &str) -> String {
        let d = format!("/tmp/flea-scan-{}-{}", tag, std::process::id());
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn reads_files_and_marks_directories() {
        let d = temp_dir("basic");
        fs::write(format!("{}/a.txt", d), "").unwrap();
        fs::create_dir(format!("{}/sub", d)).unwrap();

        let (l, _) = scan(&d, false).unwrap();
        assert_eq!(l.len(), 2);

        let mut seen_file = false;
        let mut seen_dir = false;
        for i in 0..l.len() {
            if l.name(i) == "a.txt" && !l.is_dir(i) {
                seen_file = true;
            }
            if l.name(i) == "sub" && l.is_dir(i) {
                seen_dir = true;
            }
        }
        assert!(seen_file, "expected a.txt as a file");
        assert!(seen_dir, "expected sub as a directory");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn an_empty_directory_yields_an_empty_listing() {
        let d = temp_dir("empty");
        let (l, _) = scan(&d, false).unwrap();
        assert_eq!(l.len(), 0);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_missing_directory_is_an_error_naming_the_path() {
        let e = scan("/definitely/not/here", false).unwrap_err();
        assert_eq!(e.where_, "scan");
        assert_eq!(e.path, "/definitely/not/here");
        assert!(!e.msg.is_empty());
    }

    #[test]
    fn a_directory_that_cannot_be_listed_still_answers_its_own_mode() {
        let d = temp_dir("mode");
        let locked = format!("{}/locked", d);
        fs::create_dir(&locked).unwrap();
        // Write and enter, never read: opendir is refused while stat still answers.
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o300)).unwrap();

        assert!(
            scan(&locked, false).is_err(),
            "a directory with no read bit cannot be listed"
        );
        let mode = mode_of(&locked);
        assert_eq!(
            mode & 0o777,
            0o300,
            "the stat answered the permission bits, got {:o}",
            mode
        );
        assert_eq!(
            mode & 0o170000,
            0o040000,
            "and the file-type bits say directory, got {:o}",
            mode
        );

        fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_path_that_cannot_be_stat_at_all_answers_zero() {
        assert_eq!(mode_of("/definitely/not/here"), 0);
    }

    #[test]
    fn a_dotfile_is_skipped_by_default_and_listed_when_hidden_is_true() {
        let d = temp_dir("hidden");
        fs::write(format!("{}/.dotfile", d), "").unwrap();
        fs::create_dir(format!("{}/.dotdir", d)).unwrap();
        fs::write(format!("{}/plain.txt", d), "").unwrap();

        let (visible, _) = scan(&d, false).unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible.name(0), "plain.txt");

        let (all, _) = scan(&d, true).unwrap();
        assert_eq!(all.len(), 3);
        let mut names: Vec<&str> = (0..all.len()).map(|i| all.name(i)).collect();
        names.sort();
        assert_eq!(names, [".dotdir", ".dotfile", "plain.txt"]);
        fs::remove_dir_all(&d).unwrap();
    }
}
