// The two orders the metadata pass makes possible, size and date; see AGENTS.md "Two-phase listing".
use crate::backend::listing::{Listing, Span};
use crate::backend::meta::{stat_all, Stat};
use crate::backend::sort::{name_order, SortBy};
use std::cmp::Ordering;
use std::path::Path;
use std::time::Instant;

// Stats every row, orders the listing by the key, and answers (pass ms, sort ms) for the listed line.
// The stats live for this call alone: nothing is cached, so reversing a size order stats the
// directory again at the warm cost docs/protocol.md "sort" records, and a listing sitting in name
// order, which is every listing the field measures, never carries 16 bytes a row it is not using.
pub fn sort_by_stat(l: &mut Listing, base: &Path, by: SortBy, desc: bool) -> (f64, f64) {
    let (stats, pass_ms) = stat_all(base, l);
    let t = Instant::now();
    // An index is sorted and the spans gathered after it, so Span stays the 12 bytes every listing pays.
    let mut order: Vec<u32> = (0..l.len() as u32).collect();
    order.sort_by(|&a, &b| {
        let (a, b) = (a as usize, b as usize);
        match (l.is_dir(a), l.is_dir(b)) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            // desc is the exact reverse of asc inside each group, tie-break included, as name's is.
            _ if desc => key_order(l, &stats, by, b, a),
            _ => key_order(l, &stats, by, a, b),
        }
    });
    let spans: Vec<Span> = order.iter().map(|&i| l.spans[i as usize]).collect();
    l.spans = spans;
    (pass_ms, t.elapsed().as_secs_f64() * 1000.0)
}

// Inside one group: the key, then the name, so two equal sizes list the same way every run.
// corner: a size order lists directories by name, because a directory's st_size is not a size anyone means.
fn key_order(l: &Listing, stats: &[Stat], by: SortBy, a: usize, b: usize) -> Ordering {
    let by_key = match by {
        SortBy::Size if l.is_dir(a) => Ordering::Equal,
        SortBy::Size => stats[a].size.cmp(&stats[b].size),
        SortBy::Mtime => stats[a].mtime.cmp(&stats[b].mtime),
        // sort_listing routes name to sort_by_name, so this arm never runs; Equal would still be name order.
        SortBy::Name => Ordering::Equal,
    };
    match by_key {
        Ordering::Equal => name_order(l.name(a).as_bytes(), l.name(b).as_bytes()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::testdir::TestDir;
    use std::time::{Duration, SystemTime};

    // A tree where each key disagrees with the others, the shape tests/protocol.sh builds too: 2 is the
    // larger file and the oldest entry, 3 the smaller and the newest, 11 is older than 1, and 1 holds
    // a file so its st_size is not 0. Pushed in an order that is none of the answers.
    fn tree(tag: &str) -> (TestDir, Listing) {
        let d = TestDir::new(tag);
        d.dir("1");
        d.dir("11");
        d.file("1/x", "");
        d.file("2", "00000");
        d.file("3", "0");
        stamp(&d.join("2"), 1000);
        stamp(&d.join("11"), 1001);
        stamp(&d.join("1"), 1002);
        stamp(&d.join("3"), 1003);
        let mut l = Listing::new();
        l.push("3", false);
        l.push("1", true);
        l.push("2", false);
        l.push("11", true);
        (d, l)
    }

    fn stamp(path: &Path, seconds: u64) {
        let when = SystemTime::UNIX_EPOCH + Duration::from_secs(seconds);
        std::fs::File::open(path)
            .unwrap()
            .set_modified(when)
            .unwrap();
    }

    fn names(l: &Listing) -> String {
        (0..l.len())
            .map(|i| l.name(i))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn size_lists_directories_first_by_name_then_files_by_size() {
        let (d, mut l) = tree("sizeasc");
        sort_by_stat(&mut l, d.path(), SortBy::Size, false);
        // 11 has st_size 0 and 1 does not, so a build ordering directories by st_size answers 11 1.
        assert_eq!(names(&l), "1 11 3 2");
    }

    #[test]
    fn size_descending_keeps_directories_first_and_reverses_inside_each_group() {
        let (d, mut l) = tree("sizedesc");
        sort_by_stat(&mut l, d.path(), SortBy::Size, true);
        assert_eq!(names(&l), "11 1 2 3");
    }

    #[test]
    fn mtime_orders_both_groups_by_time_directories_still_first_in_both_directions() {
        let (d, mut l) = tree("mtime");
        sort_by_stat(&mut l, d.path(), SortBy::Mtime, false);
        assert_eq!(
            names(&l),
            "11 1 2 3",
            "a build that lost the grouping answers 2 11 1 3"
        );
        sort_by_stat(&mut l, d.path(), SortBy::Mtime, true);
        assert_eq!(names(&l), "1 11 3 2");
    }

    #[test]
    fn equal_keys_fall_back_to_name_order_so_the_listing_is_the_same_every_run() {
        let d = TestDir::new("sizetie");
        let set = ["b", "a", "B", "file_10", "file_2"];
        for n in set {
            d.file(n, "same");
        }
        let mut once = Listing::new();
        let mut again = Listing::new();
        for n in set {
            once.push(n, false);
        }
        for n in set.iter().rev() {
            again.push(n, false);
        }
        sort_by_stat(&mut once, d.path(), SortBy::Size, false);
        sort_by_stat(&mut again, d.path(), SortBy::Size, false);
        assert_eq!(
            names(&once),
            "a B b file_2 file_10",
            "the tie-break is the name order, digits by value"
        );
        assert_eq!(names(&once), names(&again), "whatever readdir said");
    }

    #[test]
    fn a_vanished_row_sorts_as_empty_and_an_empty_listing_does_not_panic() {
        let d = TestDir::new("sizegone");
        d.file("real", "abc");
        let mut l = Listing::new();
        l.push("real", false);
        // Named to sort after real by name, so this can only pass on size.
        l.push("zzz-gone", false);
        sort_by_stat(&mut l, d.path(), SortBy::Size, false);
        assert_eq!(
            names(&l),
            "zzz-gone real",
            "the zeroes stat_range would send sort as the smallest"
        );
        let mut empty = Listing::new();
        let (pass, sort) = sort_by_stat(&mut empty, d.path(), SortBy::Mtime, true);
        assert_eq!(empty.len(), 0);
        assert!(pass >= 0.0 && sort >= 0.0);
    }

    #[test]
    fn the_gather_moves_spans_and_leaves_every_name_and_flag_intact() {
        let (d, mut l) = tree("gather");
        let mut before: Vec<(String, bool)> = (0..l.len())
            .map(|i| (l.name(i).to_string(), l.is_dir(i)))
            .collect();
        sort_by_stat(&mut l, d.path(), SortBy::Mtime, true);
        let mut after: Vec<(String, bool)> = (0..l.len())
            .map(|i| (l.name(i).to_string(), l.is_dir(i)))
            .collect();
        before.sort();
        after.sort();
        assert_eq!(
            before, after,
            "a sort is a permutation of the rows and nothing else"
        );
    }
}
