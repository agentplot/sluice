//! The order of a branch's one chain (6 to 8), as a pure function.

use std::collections::{BTreeMap, BTreeSet};

/// A submission as ordering sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: u64,
    pub priority: i64,
    pub after: Vec<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Order {
    /// The branch's chain, in landing order (8).
    pub chain: Vec<u64>,
    /// Submissions whose `after` is neither landed nor ahead of them (7).
    pub waiting: BTreeSet<u64>,
}

/// Order `entries` into the branch's chain.
///
/// - The order of submissions in a combination under test or judged
///   (`pinned`) is kept: priority orders only the rest (6).
/// - The rest follow by priority, then by order of submission (6), each
///   only once every `after` is landed or ahead of it in the chain (7).
pub fn order(entries: &[Entry], landed: &BTreeSet<u64>, pinned: &[Vec<u64>]) -> Order {
    let by_id: BTreeMap<u64, &Entry> = entries.iter().map(|e| (e.id, e)).collect();
    let mut chain: Vec<u64> = pinned_prefix(pinned)
        .into_iter()
        .filter(|i| by_id.contains_key(i))
        .collect();
    let mut rest: Vec<&Entry> = entries.iter().filter(|e| !chain.contains(&e.id)).collect();
    rest.sort_by_key(|e| (std::cmp::Reverse(e.priority), e.id));
    let mut n = 0;
    while n < rest.len() {
        let e = rest[n];
        if e.after
            .iter()
            .all(|a| landed.contains(a) || chain.contains(a))
        {
            chain.push(e.id);
            rest.remove(n);
            // An admission may free one waiting ahead of it in the sort:
            // start the pass again.
            n = 0;
        } else {
            n += 1;
        }
    }
    Order {
        chain,
        waiting: rest.iter().map(|e| e.id).collect(),
    }
}

/// The pinned submissions, in order: each pinned list keeps its own
/// order, and lists that are not prefixes of one another are merged by
/// order of submission, so no submission under test is put behind one
/// submitted after it (6).
fn pinned_prefix(pinned: &[Vec<u64>]) -> Vec<u64> {
    let mut lists: Vec<&Vec<u64>> = pinned.iter().filter(|l| !l.is_empty()).collect();
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

    fn e(id: u64) -> Entry {
        Entry {
            id,
            priority: 0,
            after: vec![],
        }
    }

    #[test]
    fn every_entry_is_in_one_chain_in_order_of_submission() {
        let o = order(&[e(1), e(2), e(3)], &BTreeSet::new(), &[]);
        assert_eq!(o.chain, vec![1, 2, 3]);
    }

    #[test]
    fn priority_orders_only_what_is_not_pinned() {
        let mut high = e(3);
        high.priority = 5;
        let o = order(&[e(1), e(2), high], &BTreeSet::new(), &[vec![1]]);
        assert_eq!(o.chain, vec![1, 3, 2]);
    }

    #[test]
    fn after_waits_for_landed_or_ahead() {
        let mut first = e(2);
        first.after = vec![3];
        first.priority = 1;
        let mut blocked = e(4);
        blocked.after = vec![9];
        let o = order(&[first, e(3), blocked], &BTreeSet::new(), &[]);
        assert_eq!(o.chain, vec![3, 2]);
        assert_eq!(o.waiting, BTreeSet::from([4]));
    }

    #[test]
    fn pinned_lists_merge_by_order_of_submission() {
        let o = order(
            &[e(1), e(2), e(3), e(4)],
            &BTreeSet::new(),
            &[vec![1], vec![1, 3], vec![2]],
        );
        assert_eq!(o.chain, vec![1, 2, 3, 4]);
    }
}
