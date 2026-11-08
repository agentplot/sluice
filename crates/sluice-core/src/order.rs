//! The order of a branch's one chain (6 to 8), as a pure function.

use std::collections::{BTreeMap, BTreeSet};

use crate::records::{State, Submission};

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

/// The chain of a branch's live submissions `subs` (6 to 8): those still
/// in its queue, ordered by [`order`] with `landed` and the combinations
/// `pinned` on the branch's revision, each pinned only while every one of
/// its submissions is in the queue. What is landing is left out where
/// `landing` is false, as a round leaves it, its landing under way; a
/// reader shown the queue keeps it, ahead of what is still to be tested.
pub fn chain<'a>(
    subs: impl IntoIterator<Item = &'a Submission>,
    landed: &BTreeSet<u64>,
    pinned: &[Vec<u64>],
    landing: bool,
) -> Order {
    let entries: Vec<Entry> = subs
        .into_iter()
        .filter(|s| !s.state.is_out() && (landing || s.state != State::Landing))
        .map(|s| Entry {
            id: s.id,
            priority: s.priority,
            after: s.after.clone(),
        })
        .collect();
    let ids: BTreeSet<u64> = entries.iter().map(|e| e.id).collect();
    let pinned: Vec<Vec<u64>> = pinned
        .iter()
        .filter(|c| c.iter().all(|i| ids.contains(i)))
        .cloned()
        .collect();
    order(&entries, landed, &pinned)
}

/// What is ahead of submission `id` in a branch's queue whose chain is
/// `chain`, `sub` giving each live submission by its number (6, 7): the
/// chain's submissions before it; for one waiting on an `after` (7), the
/// whole chain and then each submission it waits on that is still in the
/// queue, as a forecast reads it (64); and nothing for one out of the
/// queue or not in it.
pub fn ahead_of<'a>(
    id: u64,
    chain: &[u64],
    sub: impl Fn(u64) -> Option<&'a Submission>,
) -> Vec<u64> {
    if let Some(n) = chain.iter().position(|i| *i == id) {
        return chain[..n].to_vec();
    }
    let in_queue = |i: u64| sub(i).is_some_and(|s| !s.state.is_out());
    match sub(id) {
        Some(s) if in_queue(id) => {
            let mut ahead = chain.to_vec();
            for a in &s.after {
                if !ahead.contains(a) && in_queue(*a) {
                    ahead.push(*a);
                }
            }
            ahead
        }
        _ => Vec::new(),
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

    fn sub(id: u64, state: &str, priority: i64, after: &[u64]) -> Submission {
        serde_json::from_value(serde_json::json!({
            "schema": crate::records::SUBMISSION_SCHEMA, "id": id, "branch": "main",
            "head": format!("h{id}"), "from": { "agent": "a", "worktree": format!("w{id}") },
            "submitted": "2026-11-07T10:00:00Z", "after": after, "priority": priority,
            "attempt": 1, "state": state,
        }))
        .unwrap()
    }

    /// 6 to 8: a round's chain leaves what is landing out, and a reader's
    /// keeps it; what is out of the queue is in neither, and a combination
    /// one of whose submissions left the queue pins nothing.
    #[test]
    fn a_chain_of_submissions_keeps_landing_only_where_asked() {
        let subs = [
            sub(1, "landing", 0, &[]),
            sub(2, "failed", 0, &[]),
            sub(3, "queued", 0, &[]),
            sub(4, "queued", 1, &[]),
        ];
        let landed = BTreeSet::new();
        let pinned = [vec![1], vec![2, 3]];
        assert_eq!(chain(&subs, &landed, &pinned, false).chain, [4, 3]);
        assert_eq!(chain(&subs, &landed, &pinned, true).chain, [1, 4, 3]);
    }

    /// 6, 7, 64: ahead of one in the chain is what precedes it there;
    /// ahead of one waiting on an `after` is the whole chain and then what
    /// it waits on that is in the queue; ahead of one out of the queue, or
    /// unknown, is nothing.
    #[test]
    fn ahead_of_follows_the_chain() {
        let subs = [
            sub(1, "queued", 0, &[]),
            sub(2, "queued", 5, &[]),
            sub(3, "queued", 0, &[4]),
            sub(4, "queued", 0, &[9]),
            sub(5, "conflict", 0, &[]),
        ];
        let o = chain(&subs, &BTreeSet::new(), &[], true);
        assert_eq!(o.chain, [2, 1]);
        let get = |i: u64| subs.iter().find(|s| s.id == i);
        assert_eq!(ahead_of(1, &o.chain, get), [2]);
        assert!(ahead_of(2, &o.chain, get).is_empty());
        assert_eq!(ahead_of(3, &o.chain, get), [2, 1, 4]);
        assert_eq!(ahead_of(4, &o.chain, get), [2, 1]);
        assert!(ahead_of(5, &o.chain, get).is_empty());
        assert!(ahead_of(6, &o.chain, get).is_empty());
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
