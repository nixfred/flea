// One bounded line count, which is everything the preview column's "Lines" fact is built from. It
// sits outside metareq.rs because it shares nothing with the archive, media and subprocess machinery
// there, and because that file is at its size cap.
use std::io::Read;
use std::path::Path;

// A line count reads at most this much of a file, so a huge log costs one bounded read and says so.
pub const LINE_BUDGET: u64 = 1 << 20;

// What one bounded count learned. partial and failed are separate because they are separate facts:
// a count can stop early on a file it read fine, and a file it could not open has no count at all.
pub struct LineCount {
    pub lines: u64,
    pub partial: bool,
    pub failed: bool,
}

// A file with no trailing newline still has a last line, so the count is newlines plus one for any
// bytes after the final one; an empty file has no lines at all.
pub fn count_lines(path: &Path) -> LineCount {
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        // Permission denied, or the row vanished. Either way zero would read as "this file is empty".
        Err(_) => {
            return LineCount {
                lines: 0,
                partial: false,
                failed: true,
            }
        }
    };
    let mut buf = vec![0u8; 64 * 1024];
    let mut read: u64 = 0;
    let mut newlines: u64 = 0;
    let mut last_was_newline = true;
    let mut any = false;
    loop {
        let n = match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        any = true;
        for &b in &buf[..n] {
            if b == b'\n' {
                newlines += 1;
                last_was_newline = true;
            } else {
                last_was_newline = false;
            }
        }
        read += n as u64;
        if read >= LINE_BUDGET {
            return LineCount {
                lines: newlines,
                partial: true,
                failed: false,
            };
        }
    }
    if !any {
        return LineCount {
            lines: 0,
            partial: false,
            failed: false,
        };
    }
    let lines = newlines + if last_was_newline { 0 } else { 1 };
    LineCount {
        lines,
        partial: false,
        failed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::testdir::TestDir;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_file_that_cannot_be_opened_is_not_an_empty_one() {
        let d = TestDir::new("linecountdenied");
        let empty = count_lines(&d.file("empty.txt", ""));
        let path = d.file("denied.txt", "a\nb\nc\n");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        let denied = count_lines(&path);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            (empty.lines, empty.partial, empty.failed),
            (0, false, false),
            "an empty file really has no lines"
        );
        assert_eq!(
            (denied.lines, denied.partial, denied.failed),
            (0, false, true),
            "a file that could not be opened has no count at all"
        );
    }
}
