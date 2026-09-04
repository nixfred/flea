use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::{Duration, Instant};

// A directory over this deadline answers with what it saw, marked partial: a floor, not a wrong exact number.
const DEADLINE_MS: u64 = 2000;

pub struct DirSize {
    pub bytes: u64,
    pub partial: bool,
}

// walk_until is the testable core: a test passes an already-past deadline to force partial without waiting 2000 ms.
pub fn walk(path: &Path) -> DirSize {
    walk_until(path, Instant::now() + Duration::from_millis(DEADLINE_MS))
}

pub fn walk_until(path: &Path, deadline: Instant) -> DirSize {
    let mut bytes = 0u64;
    let mut partial = false;
    // The target's own directory entry counts too, matching what `du -s` reports for the directory itself.
    match path.symlink_metadata() {
        Ok(meta) => bytes += meta.size(),
        Err(_) => partial = true,
    }
    walk_into(path, deadline, &mut bytes, &mut partial);
    DirSize { bytes, partial }
}

// Recursion, not an explicit stack: a tree deep enough to blow it is not a shape this one box produces.
fn walk_into(path: &Path, deadline: Instant, bytes: &mut u64, partial: &mut bool) {
    if Instant::now() >= deadline {
        *partial = true;
        return;
    }
    let entries = match std::fs::read_dir(path) {
        Ok(rd) => rd,
        // Permission denied or vanished mid-walk: what was already counted stays, marked partial.
        Err(_) => {
            *partial = true;
            return;
        }
    };
    for entry in entries {
        if Instant::now() >= deadline {
            *partial = true;
            return;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                *partial = true;
                continue;
            }
        };
        // d_type is free and answers is_symlink/is_dir with no stat, matching scan.rs's own phase 1.
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => {
                *partial = true;
                continue;
            }
        };
        if file_type.is_symlink() {
            // Not followed, matching du's default; DirEntry::metadata is lstat, so only the link's own small size counts.
            if let Ok(meta) = entry.metadata() {
                *bytes += meta.size();
            } else {
                *partial = true;
            }
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => {
                *partial = true;
                continue;
            }
        };
        *bytes += meta.size();
        if file_type.is_dir() {
            walk_into(&entry.path(), deadline, bytes, partial);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::os::unix::fs::PermissionsExt;

    fn fixture(tag: &str) -> String {
        let d = format!("/tmp/flea-dirsize-{}-{}", tag, std::process::id());
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn an_empty_directory_counts_only_its_own_entry() {
        let d = fixture("empty");
        let result = walk(Path::new(&d));
        let own = fs::symlink_metadata(&d).unwrap().size();
        assert_eq!(result.bytes, own);
        assert!(!result.partial);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn files_and_nested_directories_sum_together() {
        let d = fixture("nested");
        fs::write(format!("{}/a.txt", d), "abc").unwrap();
        fs::create_dir(format!("{}/sub", d)).unwrap();
        fs::write(format!("{}/sub/b.txt", d), "de").unwrap();
        let result = walk(Path::new(&d));
        let expected = fs::symlink_metadata(&d).unwrap().size()
            + fs::symlink_metadata(format!("{}/a.txt", d)).unwrap().size()
            + fs::symlink_metadata(format!("{}/sub", d)).unwrap().size()
            + fs::symlink_metadata(format!("{}/sub/b.txt", d))
                .unwrap()
                .size();
        assert_eq!(result.bytes, expected);
        assert!(!result.partial);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_symlink_is_not_followed() {
        // The target lives outside the walked tree, so a followed link is the only way its 100,000 bytes could ever show up.
        let d = fixture("symlink");
        let target = fixture("symlink-target");
        fs::write(format!("{}/huge.bin", target), vec![0u8; 100_000]).unwrap();
        symlink(&target, format!("{}/link", d)).unwrap();
        let result = walk(Path::new(&d));
        let expected = fs::symlink_metadata(&d).unwrap().size()
            + fs::symlink_metadata(format!("{}/link", d)).unwrap().size();
        assert_eq!(result.bytes, expected);
        assert!(
            result.bytes < 100_000,
            "the symlink's own small size counts, not the target it points at"
        );
        fs::remove_dir_all(&d).unwrap();
        fs::remove_dir_all(&target).unwrap();
    }

    #[test]
    fn an_expired_deadline_marks_partial_and_keeps_what_it_saw() {
        let d = fixture("deadline");
        fs::write(format!("{}/a.txt", d), "abc").unwrap();
        let past = Instant::now() - Duration::from_secs(1);
        let result = walk_until(Path::new(&d), past);
        assert!(result.partial);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_permission_denied_subtree_adds_what_it_saw_and_marks_partial() {
        let d = fixture("denied");
        fs::write(format!("{}/visible.txt", d), "abc").unwrap();
        fs::create_dir(format!("{}/locked", d)).unwrap();
        fs::write(format!("{}/locked/hidden.txt", d), "xyz").unwrap();
        fs::set_permissions(format!("{}/locked", d), fs::Permissions::from_mode(0o000)).unwrap();
        let result = walk(Path::new(&d));
        assert!(
            result.partial,
            "a subtree it could not read must mark partial"
        );
        let expected_min = fs::symlink_metadata(&d).unwrap().size()
            + fs::symlink_metadata(format!("{}/visible.txt", d))
                .unwrap()
                .size();
        assert!(
            result.bytes >= expected_min,
            "what the walk could see must still be counted"
        );
        fs::set_permissions(format!("{}/locked", d), fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_missing_directory_answers_zero_and_partial_rather_than_a_panic() {
        let result = walk(Path::new("/definitely/not/here/flea-dirsize-test"));
        assert_eq!(result.bytes, 0);
        assert!(result.partial);
    }
}
