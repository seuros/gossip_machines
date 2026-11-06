#![allow(clippy::expect_used)]
use super::*;
#[test]
fn fanout_is_bounded_and_untried_peers_are_not_starved() {
    let mut rumor = Rumor::new([NodeId(1), NodeId(2), NodeId(3)], NonZeroU32::MIN).expect("peers");
    let first = rumor.round::<2>(0);
    assert!(first[0].is_some());
    assert_ne!(first[0], first[1]);
    let second = rumor.round::<2>(0);
    assert!(second[0].is_some());
    assert!(!first.contains(&second[0]));
    assert_eq!(second[1], None);
    assert_eq!(rumor.round::<1>(1), [None]);
    assert!(!rumor.delivered());
    for peer in [NodeId(1), NodeId(2), NodeId(3)] {
        assert!(rumor.acknowledge(peer));
    }
    assert!(rumor.delivered());
    assert!(!rumor.acknowledge(NodeId(9)));
}
#[test]
fn duplicate_memberships_reject_and_acks_stop_sends() {
    assert!(Rumor::new([NodeId(1); 2], NonZeroU32::MIN).is_none());
    let mut rumor = Rumor::new([NodeId(1)], NonZeroU32::MIN).expect("peers");
    assert!(rumor.acknowledge(NodeId(1)));
    assert_eq!(rumor.round::<1>(0), [None]);
    assert!(Rumor::new([], NonZeroU32::MIN).expect("empty").delivered());
}

#[test]
fn sparse_capacity_does_not_invent_members_or_count_inactive_ack_slots() {
    let peers = [NodeId(0), NodeId(u64::MAX)];
    let mut rumor = Rumor::<64>::from_slice(&peers, NonZeroU32::MIN).expect("sparse peers");
    assert_eq!(rumor.peers(), &peers);
    assert_eq!(rumor.attempt_limit(), 1);
    assert_eq!(rumor.status(NodeId(0)), Some((0, false)));
    assert_eq!(rumor.status(NodeId(1)), None);
    let targets = rumor.round::<3>(u64::MAX);
    assert!(targets[..2].contains(&Some(NodeId(0))));
    assert!(targets[..2].contains(&Some(NodeId(u64::MAX))));
    assert_eq!(targets[2], None);
    assert!(rumor.exhausted());
    assert!(!rumor.delivered());
    for peer in peers {
        assert!(rumor.acknowledge(peer));
        assert_eq!(rumor.status(peer), Some((1, true)));
    }
    assert!(rumor.delivered());
    assert!(Rumor::<1>::from_slice(&peers, NonZeroU32::MIN).is_none());
}

#[test]
fn empty_membership_is_delivered_without_sending_padding() {
    let mut rumor = Rumor::<64>::from_slice(&[], NonZeroU32::MIN).expect("empty");
    assert!(rumor.delivered());
    assert!(rumor.exhausted());
    assert_eq!(rumor.round::<3>(0), [None; 3]);
    assert!(!rumor.acknowledge(NodeId(0)));
}
