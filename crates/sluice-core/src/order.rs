//! Lanes and the order of each chain (6 to 9), as a pure function.

use std::collections::{BTreeMap, BTreeSet};

/// A submission as ordering sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: u64,
    pub priority: i64,
    pub after: Vec<u64>,
    /// `None` when its affected parts are unknown (9).
    pub parts: Option<BTreeSet<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Order {
    /// One chain per lane, each in landing order; chains are ordered by
    /// their oldest submission.
    pub chains: Vec<Vec<u64>>,
    /// Submissions whose `after` is neither landed nor ahead of them (7).
    pub waiting: BTreeSet<u64>,
}

/// Order `entries` into chains.
///
/// - Lanes are the connected components of overlapping parts; a
///   submission whose parts are unknown overlaps every other (8, 9).
/// - The order of submissions in a combination under test or judged
///   (`pinned`) is kept: priority orders only the rest (6).
/// - The rest follow by priority, then by order of submission (6), each
///   only once every `after` is landed or ahead of it in its chain (7).
pub fn order(entries: &[Entry], landed: &BTreeSet<u64>, pinned: &[Vec<u64>]) -> Order {
    let ids: Vec<u64> = entries.iter().map(|e| e.id).collect();
    let by_id: BTreeMap<u64, &Entry> = entries.iter().map(|e| (e.id, e)).collect();

    // Union-find over the entries.
    let mut parent: BTreeMap<u64, u64> = ids.iter().map(|&i| (i, i)).collect();
    fn find(parent: &mut BTreeMap<u64, u64>, i: u64) -> u64 {
        let p = parent[&i];
        if p == i {
            return i;
        }
        let r = find(parent, p);
        parent.insert(i, r);
        r
    }
    for (n, a) in entries.iter().enumerate() {
        for b in &entries[n + 1..] {
            let overlap = match (&a.parts, &b.parts) {
                (Some(x), Some(y)) => !x.is_disjoint(y),
                _ => true,
            };
            if overlap {
                let (ra, rb) = (find(&mut parent, a.id), find(&mut parent, b.id));
                if ra != rb {
                    parent.insert(ra.max(rb), ra.min(rb));
                }
            }
        }
    }
    let mut lanes: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
    for &i in &ids {
        let r = find(&mut parent, i);
        lanes.entry(r).or_default().insert(i);
    }

    let mut out = Order::default();
    for lane in lanes.values() {
        let mut chain = pinned_prefix(lane, pinned);
        let mut rest: Vec<&Entry> = lane
            .iter()
            .filter(|i| !chain.contains(i))
            .map(|i| by_id[i])
            .collect();
        rest.sort_by_key(|e| (std::cmp::Reverse(e.priority), e.id));
        loop {
            let before = rest.len();
            let mut n = 0;
            while n < rest.len() {
                let e = rest[n];
                if e.after
                    .iter()
                    .all(|a| landed.contains(a) || chain.contains(a))
                {
                    chain.push(e.id);
                    rest.remove(n);
                    // An admission may free one waiting ahead of it in
                    // the sort: start the pass again.
                    n = 0;
                } else {
                    n += 1;
                }
            }
            if rest.len() == before {
                break;
            }
        }
        out.waiting.extend(rest.iter().map(|e| e.id));
        if !chain.is_empty() {
            out.chains.push(chain);
        }
    }
    out.chains.sort_by_key(|c| c[0]);
    out
}

/// The pinned submissions of one lane, in order: each pinned list keeps
/// its own order, and lists of lanes that have since joined are merged
/// by order of submission, so no submission under test is put behind
/// one submitted after it (6).
fn pinned_prefix(lane: &BTreeSet<u64>, pinned: &[Vec<u64>]) -> Vec<u64> {
    let mut lists: Vec<&Vec<u64>> = pinned
        .iter()
        .filter(|l| !l.is_empty() && l.iter().all(|i| lane.contains(i)))
        .collect();
    lists.sort_by_key(|l| std::cmp::Reverse(l.len()));
    let mut kept: Vec<Vec<u64>> = Vec::new();
    for l in lists {
        if !kept.iter().any(|k| k.starts_with(l)) {
            kept.push(l.clone());
        }
    }
    let mut out: Vec<u64> = Vec::new();
    let mut heads: Vec<usize> = vec![0; kept.len()];
    loop {
        let next = kept
            .iter()
            .enumerate()
            .filter_map(|(n, l)| l.get(heads[n]).map(|&id| (id, n)))
            .min();
        let Some((id, n)) = next else { break };
        heads[n] += 1;
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(id: u64, parts: Option<&[&str]>) -> Entry {
        Entry {
            id,
            priority: 0,
            after: vec![],
            parts: parts.map(|p| p.iter().map(|s| s.to_string()).collect()),
        }
    }

    #[test]
    fn disjoint_parts_are_separate_lanes() {
        let o = order(
            &[
                e(1, Some(&["a"])),
                e(2, Some(&["b"])),
                e(3, Some(&["a", "c"])),
            ],
            &BTreeSet::new(),
            &[],
        );
        assert_eq!(o.chains, vec![vec![1, 3], vec![2]]);
    }

    #[test]
    fn unknown_parts_join_every_lane() {
        let o = order(
            &[e(1, Some(&["a"])), e(2, Some(&["b"])), e(3, None)],
            &BTreeSet::new(),
            &[],
        );
        assert_eq!(o.chains, vec![vec![1, 2, 3]]);
    }

    #[test]
    fn priority_orders_only_what_is_not_pinned() {
        let mut high = e(3, Some(&["a"]));
        high.priority = 5;
        let o = order(
            &[e(1, Some(&["a"])), e(2, Some(&["a"])), high],
            &BTreeSet::new(),
            &[vec![1]],
        );
        assert_eq!(o.chains, vec![vec![1, 3, 2]]);
    }

    #[test]
    fn after_waits_for_landed_or_ahead() {
        let mut first = e(2, Some(&["a"]));
        first.after = vec![3];
        first.priority = 1;
        let mut blocked = e(4, Some(&["b"]));
        blocked.after = vec![9];
        let o = order(&[first, e(3, Some(&["a"])), blocked], &BTreeSet::new(), &[]);
        assert_eq!(o.chains, vec![vec![3, 2]]);
        assert_eq!(o.waiting, BTreeSet::from([4]));
    }

    #[test]
    fn joined_lanes_keep_order_of_submission_among_pinned() {
        let o = order(
            &[
                e(1, Some(&["a"])),
                e(2, Some(&["b"])),
                e(3, Some(&["a"])),
                e(4, None),
            ],
            &BTreeSet::new(),
            &[vec![1], vec![1, 3], vec![2]],
        );
        assert_eq!(o.chains, vec![vec![1, 2, 3, 4]]);
    }
}
