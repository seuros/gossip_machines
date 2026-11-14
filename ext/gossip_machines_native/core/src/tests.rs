#![allow(clippy::expect_used, clippy::unwrap_used, missing_docs)]
extern crate std;
use super::*;
use std::vec::Vec;

const ONCE: NonZeroU32 = NonZeroU32::MIN;
const ACKNOWLEDGED: Result<Acknowledgement, Refusal> = Ok(Acknowledgement::Acknowledged);
const DUPLICATE: Result<Acknowledgement, Refusal> = Ok(Acknowledgement::Duplicate);
const UNKNOWN: Result<Acknowledgement, Refusal> = Err(Refusal::UnknownPeer);
/// What a round with nothing to send returns.
const NOBODY: [NodeId; 0] = [];
/// A buffer filler that is no peer of any test membership.
const UNTOUCHED: NodeId = NodeId(999);

fn ids(raw: &[u64]) -> Vec<NodeId> {
    raw.iter().copied().map(NodeId).collect()
}

fn limit(attempts: u32) -> NonZeroU32 {
    NonZeroU32::new(attempts).expect("positive")
}

/// One round into a fresh buffer of `fanout`; only the peers written.
fn round<const N: usize>(rumor: &mut Rumor<N>, seed: u64, fanout: usize) -> Vec<NodeId> {
    let mut out = std::vec![UNTOUCHED; fanout];
    let written = rumor.round(seed, &mut out);
    assert!(out[written..].iter().all(|slot| *slot == UNTOUCHED));
    out.truncate(written);
    out
}

const fn status(attempts: u32, acknowledged: bool) -> Status {
    Status {
        attempts,
        acknowledged,
    }
}

/// Peers 1 and 2, each sent at most `attempts` times.
fn pair(attempts: u32) -> Rumor<2> {
    Rumor::new(&ids(&[1, 2]), limit(attempts)).expect("peers")
}

#[test]
fn fanout_is_bounded_and_untried_peers_are_not_starved() {
    let mut rumor = Rumor::<3>::new(&ids(&[1, 2, 3]), ONCE).expect("peers");
    let first = round(&mut rumor, 0, 2);
    assert_eq!(first.len(), 2);
    assert_ne!(first[0], first[1]);
    let second = round(&mut rumor, 0, 2);
    assert_eq!(second.len(), 1);
    assert!(!first.contains(&second[0]));
    assert!(rumor.exhausted());
    assert!(!rumor.delivered());
    for peer in ids(&[1, 2, 3]) {
        assert_eq!(rumor.acknowledge(peer), ACKNOWLEDGED);
    }
    assert!(rumor.delivered());
}

#[test]
fn a_round_with_nobody_to_send_to_answers_zero_and_changes_nothing() {
    // Exhausted, not delivered.
    let mut rumor = pair(1);
    assert_eq!(round(&mut rumor, 3, 4).len(), 2);
    assert!(rumor.exhausted() && !rumor.delivered());
    let spent = rumor.clone();
    assert_eq!(round(&mut rumor, 4, 4), NOBODY);
    assert_eq!(rumor, spent);
    // Delivered, attempts left over.
    let mut rumor = pair(5);
    assert_eq!(rumor.acknowledge(NodeId(1)), ACKNOWLEDGED);
    assert_eq!(rumor.acknowledge(NodeId(2)), ACKNOWLEDGED);
    assert!(rumor.delivered() && rumor.exhausted());
    let done = rumor.clone();
    assert_eq!(round(&mut rumor, 0, 4), NOBODY);
    assert_eq!(rumor, done);
    // Empty membership: delivered from the start.
    let mut empty = Rumor::<4>::new(&[], ONCE).expect("empty");
    assert!(empty.delivered() && empty.exhausted());
    let fresh = empty.clone();
    assert_eq!(round(&mut empty, 0, 3), NOBODY);
    assert_eq!(empty, fresh);
    // No room in the buffer: nobody is charged for a send that was not planned.
    let mut rumor = pair(1);
    let fresh = rumor.clone();
    assert_eq!(rumor.round(0, &mut []), 0);
    assert_eq!(rumor, fresh);
}

#[test]
fn exactly_the_peers_written_are_charged_one_attempt_each() {
    let peers = ids(&[1, 2, 3, 4, 5]);
    let mut rumor = Rumor::<8>::new(&peers, limit(2)).expect("peers");
    let mut sends = 0;
    for seed in 0..10 {
        let before: Vec<u32> = peers
            .iter()
            .map(|peer| rumor.status(*peer).expect("member").attempts)
            .collect();
        let chosen = round(&mut rumor, seed, 3);
        sends += chosen.len();
        for (peer, before) in peers.iter().zip(before) {
            let charged = u32::from(chosen.contains(peer));
            assert_eq!(rumor.status(*peer), Some(status(before + charged, false)));
        }
    }
    // Five peers, two attempts each: ten sends, in rounds of 3, 3, 3, 1, 0...
    assert_eq!(sends, 10);
    assert!(rumor.exhausted());
}

#[test]
fn acknowledge_names_first_repeat_and_unknown() {
    let mut rumor = pair(1);
    let fresh = rumor.clone();
    assert_eq!(rumor.acknowledge(NodeId(9)), UNKNOWN);
    assert_eq!(rumor, fresh);
    // Acknowledging a peer never sent to is honoured: it heard elsewhere.
    assert_eq!(rumor.acknowledge(NodeId(1)), ACKNOWLEDGED);
    assert_eq!(rumor.status(NodeId(1)), Some(status(0, true)));
    let held = rumor.clone();
    assert_eq!(rumor.acknowledge(NodeId(1)), DUPLICATE);
    assert_eq!(rumor.acknowledge(NodeId(9)), UNKNOWN);
    assert_eq!(rumor, held);
    // An acknowledged peer is never selected; a spent one may still ack late.
    assert_eq!(round(&mut rumor, 0, 2), [NodeId(2)]);
    assert!(rumor.exhausted() && !rumor.delivered());
    assert_eq!(rumor.acknowledge(NodeId(2)), ACKNOWLEDGED);
    assert_eq!(rumor.status(NodeId(2)), Some(status(1, true)));
    assert!(rumor.delivered());
    let done = rumor.clone();
    assert_eq!(rumor.acknowledge(NodeId(2)), DUPLICATE);
    assert_eq!(rumor, done);
}

#[test]
fn memberships_reject_duplicates_and_overflow() {
    for raw in [&[1, 1][..], &[1, 2, 3]] {
        assert_eq!(Rumor::<2>::new(&ids(raw), ONCE), None);
    }
    assert_eq!(Rumor::<0>::new(&ids(&[1]), ONCE), None);
    let none = Rumor::<0>::new(&[], ONCE).expect("empty");
    assert!(none.delivered());
    assert_eq!(none.capacity(), 0);
}

#[test]
fn sparse_capacity_does_not_invent_members_or_count_inactive_ack_slots() {
    let peers = [NodeId(0), NodeId(u64::MAX)];
    let mut rumor = Rumor::<64>::new(&peers, ONCE).expect("sparse peers");
    assert_eq!(rumor.peers(), &peers);
    assert_eq!(rumor.attempt_limit(), ONCE);
    assert_eq!(rumor.capacity(), Rumor::<64>::CAPACITY);
    assert_eq!(Rumor::<64>::CAPACITY, 64);
    assert_eq!(rumor.status(NodeId(0)), Some(status(0, false)));
    assert_eq!(rumor.status(NodeId(1)), None);
    // The default id fills unused capacity but is a member only if supplied.
    let mut other = Rumor::<64>::new(&[NodeId(7)], ONCE).expect("peer");
    assert_eq!(other.acknowledge(NodeId::default()), UNKNOWN);
    assert_eq!(other.status(NodeId::default()), None);
    let mut targets = round(&mut rumor, u64::MAX, 3);
    targets.sort();
    assert_eq!(targets, peers);
    assert!(rumor.exhausted());
    assert!(!rumor.delivered());
    for peer in peers {
        assert_eq!(rumor.acknowledge(peer), ACKNOWLEDGED);
        assert_eq!(rumor.status(peer), Some(status(1, true)));
    }
    assert!(rumor.delivered());
}

#[test]
fn refusal_is_an_error_with_a_message() {
    fn message(error: &dyn core::error::Error) -> std::string::String {
        std::format!("{error}")
    }
    assert!(message(&Refusal::UnknownPeer).contains("membership"));
}

mod model;
