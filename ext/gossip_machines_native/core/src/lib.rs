#![no_std]
#![doc = include_str!("../README.md")]
use core::num::NonZeroU32;
use placement_machines::{NodeId, score};
/// Send/ack state for one rumor and one fixed peer membership.
#[derive(Debug)]
pub struct Rumor<const N: usize> {
    peers: [NodeId; N],
    len: usize,
    attempts: [u32; N],
    acknowledged: [bool; N],
    limit: NonZeroU32,
}
impl<const N: usize> Rumor<N> {
    /// Reject duplicate IDs; callers exclude the sender from this peer list.
    #[must_use]
    pub fn new(peers: [NodeId; N], limit: NonZeroU32) -> Option<Self> {
        Self::from_slice(&peers, limit)
    }

    /// Retain at most `N` real peers; inactive capacity never participates.
    ///
    /// Duplicate IDs or an oversized membership reject the configuration.
    #[must_use]
    pub fn from_slice(peers: &[NodeId], limit: NonZeroU32) -> Option<Self> {
        if peers.len() > N {
            return None;
        }
        for (index, peer) in peers.iter().enumerate() {
            if peers[..index].contains(peer) {
                return None;
            }
        }
        let mut storage = [NodeId(0); N];
        storage[..peers.len()].copy_from_slice(peers);
        Some(Self {
            peers: storage,
            len: peers.len(),
            attempts: [0; N],
            acknowledged: [false; N],
            limit,
        })
    }
    /// Charge one fanout, never repeating a peer within this round.
    pub fn round<const F: usize>(&mut self, seed: u64) -> [Option<NodeId>; F] {
        let mut selected = [None; F];
        for slot in 0..F {
            let candidate = (0..self.len)
                .filter(|&i| {
                    !self.acknowledged[i]
                        && self.attempts[i] < self.limit.get()
                        && !selected[..slot].contains(&Some(self.peers[i]))
                })
                .min_by_key(|&i| {
                    (
                        self.attempts[i],
                        core::cmp::Reverse(score(seed, self.peers[i])),
                        self.peers[i],
                    )
                });
            if let Some(index) = candidate {
                self.attempts[index] += 1;
                selected[slot] = Some(self.peers[index]);
            }
        }
        selected
    }
    /// Record an authenticated, rumor-correlated peer acknowledgement.
    #[must_use]
    pub fn acknowledge(&mut self, peer: NodeId) -> bool {
        let Some(index) = self.index(peer) else {
            return false;
        };
        self.acknowledged[index] = true;
        true
    }
    /// All peers acknowledged, not merely exhausted their attempts.
    #[must_use]
    pub fn delivered(&self) -> bool {
        self.acknowledged[..self.len].iter().all(|ack| *ack)
    }

    /// Exact caller membership, without inactive capacity.
    #[must_use]
    pub fn peers(&self) -> &[NodeId] {
        &self.peers[..self.len]
    }

    fn index(&self, peer: NodeId) -> Option<usize> {
        self.peers[..self.len].iter().position(|id| *id == peer)
    }

    /// Per-peer attempt count and acknowledgement state, or `None` if unknown.
    #[must_use]
    pub fn status(&self, peer: NodeId) -> Option<(u32, bool)> {
        self.index(peer)
            .map(|index| (self.attempts[index], self.acknowledged[index]))
    }

    /// No peer is eligible for another send; this does not imply delivery.
    #[must_use]
    pub fn exhausted(&self) -> bool {
        (0..self.len)
            .all(|index| self.acknowledged[index] || self.attempts[index] >= self.limit.get())
    }

    /// Maximum charged sends for each peer.
    #[must_use]
    pub const fn attempt_limit(&self) -> u32 {
        self.limit.get()
    }
}
#[cfg(test)]
mod tests;
