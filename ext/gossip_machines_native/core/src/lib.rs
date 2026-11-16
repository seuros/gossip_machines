#![no_std]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), deny(clippy::indexing_slicing))]
use core::cmp::Reverse;
use core::fmt;
use core::num::NonZeroU32;

use placement_machines::score;

/// A peer's identity: the sibling type every method here takes and returns.
pub use placement_machines::NodeId;

/// Why a call changed nothing.
///
/// [`Rumor::acknowledge`] answers [`UnknownPeer`](Self::UnknownPeer). Nothing
/// else refuses: [`Rumor::round`] is a dispatch, and "nobody to send to" is
/// its answer zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The peer is not in this rumor's membership.
    UnknownPeer,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnknownPeer => "the peer is not in this rumor's membership",
        })
    }
}

impl core::error::Error for Refusal {}

/// What an honoured [`Rumor::acknowledge`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acknowledgement {
    /// The peer had not acknowledged; it is now acknowledged and never
    /// selected again.
    Acknowledged,
    /// An idempotent repeat: the peer had already acknowledged. Nothing changed.
    Duplicate,
}

/// One peer's two ledgers, as [`Rumor::status`] reads them.
///
/// Every pair of fields is a valid reading, so the fields are public and there
/// is no constructor to pass: the bound on `attempts` is one rumor's
/// [`Rumor::attempt_limit`], not a property of the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    /// Sends charged to the peer so far, at most [`Rumor::attempt_limit`].
    pub attempts: u32,
    /// Whether the peer acknowledged the rumor.
    pub acknowledged: bool,
}

/// Send/ack state for one rumor and one fixed peer membership of at most `N`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rumor<const N: usize> {
    peers: [NodeId; N],
    len: usize,
    attempts: [u32; N],
    acknowledged: [bool; N],
    attempt_limit: NonZeroU32,
}

impl<const N: usize> Rumor<N> {
    /// Most peers a membership can hold: `N`, the compiled capacity.
    pub const CAPACITY: usize = N;

    /// A rumor nobody has been sent or has acknowledged, for exactly `peers`.
    ///
    /// Callers leave the sender out of `peers`. An empty membership is valid
    /// and already delivered. `None` when `peers` holds more than `N` ids or
    /// repeats one.
    #[must_use]
    pub fn new(peers: &[NodeId], attempt_limit: NonZeroU32) -> Option<Self> {
        if peers.len() > N {
            return None;
        }
        for (index, peer) in peers.iter().enumerate() {
            if peers.iter().take(index).any(|earlier| earlier == peer) {
                return None;
            }
        }
        let mut storage = [NodeId::default(); N];
        // `peers.len() <= N`, so every peer finds a slot.
        for (slot, peer) in storage.iter_mut().zip(peers) {
            *slot = *peer;
        }
        Some(Self {
            peers: storage,
            len: peers.len(),
            attempts: [0; N],
            acknowledged: [false; N],
            attempt_limit,
        })
    }

    /// Plan one round: pick up to `out.len()` distinct eligible peers, charge
    /// each one attempt, write them to the front of `out` and return how many.
    ///
    /// A peer is eligible while it has not acknowledged and has fewer than
    /// [`attempt_limit`](Self::attempt_limit) attempts. Fewest attempts go
    /// first; `seed` settles ties by placement score, highest first, then the
    /// lower id. Entries of `out` past the returned count are left untouched.
    ///
    /// This is a dispatch and never refuses. Exactly the peers written are
    /// charged, one attempt each, and nothing else is counted: there is no
    /// round counter. So a round that writes nothing (no eligible peer because
    /// the rumor is delivered or exhausted, an empty membership, or an empty
    /// `out`) answers `0` and changes nothing.
    pub fn round(&mut self, seed: u64, out: &mut [NodeId]) -> usize {
        let limit = self.attempt_limit.get();
        let mut written = 0;
        // `written <= out.len()` throughout, so the split always succeeds and
        // the loop ends when `rest` runs out.
        while let Some((chosen, rest)) = out.split_at_mut_checked(written) {
            let Some(slot) = rest.first_mut() else {
                break;
            };
            let candidate = self
                .peers
                .iter()
                .zip(self.attempts.iter_mut())
                .zip(&self.acknowledged)
                .take(self.len)
                .filter(|((peer, attempts), acknowledged)| {
                    !**acknowledged && **attempts < limit && !chosen.contains(peer)
                })
                .min_by_key(|((peer, attempts), _)| {
                    (**attempts, Reverse(score(seed, **peer)), **peer)
                });
            let Some(((peer, attempts), _)) = candidate else {
                break;
            };
            // Eligible means `attempts < limit <= u32::MAX`: no overflow.
            *attempts += 1;
            *slot = *peer;
            // `written < out.len()` here, so no overflow.
            written += 1;
        }
        written
    }

    /// Record an authenticated, rumor-correlated acknowledgement from `peer`.
    ///
    /// [`Acknowledgement::Acknowledged`] the first time, whether or not the
    /// peer was ever selected. A repeat is the effect asked for already
    /// holding, which the peer's acknowledged bit proves:
    /// [`Acknowledgement::Duplicate`], and nothing changes. Attempt counts are
    /// never touched.
    ///
    /// # Errors
    /// [`Refusal::UnknownPeer`] when `peer` is not in the membership. Nothing
    /// changes.
    pub fn acknowledge(&mut self, peer: NodeId) -> Result<Acknowledgement, Refusal> {
        let acknowledged = self
            .peers
            .iter()
            .zip(self.acknowledged.iter_mut())
            .take(self.len)
            .find_map(|(id, acknowledged)| (*id == peer).then_some(acknowledged))
            .ok_or(Refusal::UnknownPeer)?;
        if *acknowledged {
            return Ok(Acknowledgement::Duplicate);
        }
        *acknowledged = true;
        Ok(Acknowledgement::Acknowledged)
    }

    /// All peers acknowledged, not merely exhausted their attempts.
    #[must_use]
    pub fn delivered(&self) -> bool {
        self.ledgers().all(|(_, _, acknowledged)| acknowledged)
    }

    /// No peer is eligible for another send; this does not imply delivery.
    ///
    /// Once true, every [`round`](Self::round) answers `0`.
    #[must_use]
    pub fn exhausted(&self) -> bool {
        let limit = self.attempt_limit.get();
        self.ledgers()
            .all(|(_, attempts, acknowledged)| acknowledged || attempts >= limit)
    }

    /// Exact caller membership, in the caller's order, without inactive capacity.
    #[must_use]
    pub fn peers(&self) -> &[NodeId] {
        // `len <= N` by construction; were it broken the membership reads
        // as empty rather than panicking.
        self.peers.get(..self.len).unwrap_or_default()
    }

    /// Per-peer attempt count and acknowledgement state, or `None` if unknown.
    #[must_use]
    pub fn status(&self, peer: NodeId) -> Option<Status> {
        self.ledgers()
            .find(|(id, _, _)| *id == peer)
            .map(|(_, attempts, acknowledged)| Status {
                attempts,
                acknowledged,
            })
    }

    /// Maximum charged sends for each peer.
    #[must_use]
    pub const fn attempt_limit(&self) -> NonZeroU32 {
        self.attempt_limit
    }

    /// Most peers a membership can hold: [`Self::CAPACITY`], which is `N`.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        Self::CAPACITY
    }

    /// Each member with its two ledgers, in membership order.
    fn ledgers(&self) -> impl Iterator<Item = (NodeId, u32, bool)> + '_ {
        self.peers
            .iter()
            .zip(&self.attempts)
            .zip(&self.acknowledged)
            .take(self.len)
            .map(|((peer, attempts), acknowledged)| (*peer, *attempts, *acknowledged))
    }
}

#[cfg(test)]
mod tests;
