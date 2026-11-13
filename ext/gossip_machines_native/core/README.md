# gossip-machines

> **AMS Demers**, NCC-4210, *Demers*-class relay vessel.

Engine-room flight manual: plan bounded rumor relays across neighboring ships.

Allocation-free `no_std` planner for one rumor under fixed unique peer IDs.
Each round selects unacknowledged peers up to the caller's fanout, preferring
fewer previous sends and then placement scores. Supply a changing random seed
for randomized peer choice. Send attempts are bounded per peer.

Planning charges an attempt, not successful delivery. Ack state and attempt
exhaustion are distinct. Empty memberships are already acknowledged. The caller
authenticates and correlates acks to the rumor/membership epoch. No discovery,
failure detection, SWIM, anti-entropy reconciliation, transport, or consensus.
An exhausted rumor may still be undelivered; it is not globally committed.

```rust
use core::num::NonZeroU32;
use gossip_machines::{Acknowledgement, NodeId, Refusal, Rumor, Status};

let peers = [NodeId(10), NodeId(20), NodeId(30)];
let mut rumor = Rumor::<8>::new(&peers, NonZeroU32::MIN).expect("distinct peers");

// Fanout two: the buffer is the fanout. Only the first `n` entries are peers.
let mut out = [NodeId::default(); 2];
assert_eq!(rumor.round(7, &mut out), 2);
let (first, second) = (out[0], out[1]);
assert_eq!(rumor.round(7, &mut out), 1);
let third = out[0];
assert!(first != second && second != third && first != third);

// Everyone has been tried once: exhausted, and still not delivered.
assert!(rumor.exhausted() && !rumor.delivered());
let spent = rumor.clone();
assert_eq!(rumor.round(8, &mut out), 0);
assert_eq!(rumor, spent);

assert_eq!(rumor.acknowledge(first), Ok(Acknowledgement::Acknowledged));
assert_eq!(rumor.acknowledge(first), Ok(Acknowledgement::Duplicate));
assert_eq!(rumor.acknowledge(NodeId(99)), Err(Refusal::UnknownPeer));
assert_eq!(rumor.status(first), Some(Status { attempts: 1, acknowledged: true }));
assert_eq!(rumor.status(NodeId(99)), None);
```

## Contract

- **Membership.** `Rumor<N>::new(peers: &[NodeId], attempt_limit: NonZeroU32)`
  holds at most `N` peers, fixed for the rumor's life. It answers `None` for
  more than `N` ids or a repeated id; an empty slice is valid and already
  delivered. Leave the sender out. `capacity()` is `N`.
- **Sibling seam.** A peer is a `placement_machines::NodeId`, re-exported here
  as `gossip_machines::NodeId`; every method takes or returns it and callers
  need no direct dependency on `placement-machines`. Ids are chosen by the
  caller: this core issues no handles, so it has no scopes and nothing to
  resume.
- **Rounds are a dispatch.** `round(seed, out: &mut [NodeId]) -> usize` picks
  up to `out.len()` distinct eligible peers, writes them to the front of `out`,
  returns how many and leaves the rest of the buffer untouched. `NodeId`
  implements `Default` so a buffer is `[NodeId::default(); F]`; the default is
  a placeholder, not a sentinel (`NodeId(0)` can be a real peer), so read only
  the entries reported. A peer is eligible while it has not acknowledged and
  has fewer than `attempt_limit` attempts. Fewest attempts go first; `seed`
  breaks ties by `placement_machines::score(seed, peer)`, highest first, then
  the lower id.
- **Attempt accounting.** A round charges exactly the peers it writes, one
  attempt each, at the moment it selects them, whether or not the send later
  succeeds. Nothing else is counted: there is no round counter and no budget
  shared between peers. A round that writes nothing therefore consumes
  nothing. An attempt count never exceeds `attempt_limit` and is never
  refunded or reset.
- **Nothing to send is an answer, not a refusal.** Once the rumor is delivered
  or exhausted (or the membership or `out` is empty), `round` answers `0` and
  the rumor is bit-for-bit unchanged. `round` never refuses: a caller that
  wants to know why nothing came back asks `delivered()` and `exhausted()`.
- **Acknowledgements.** `acknowledge(peer) -> Result<Acknowledgement, Refusal>`.
  `Acknowledged` the first time, for any member, tried or not, spent or not.
  A repeat is honoured as `Acknowledgement::Duplicate` and changes nothing:
  the peer's acknowledged bit is kept for the rumor's whole life, so the
  repeat is always provable, and an acknowledgement has no payload that could
  differ. An acknowledgement never changes an attempt count.
- **Refusals change nothing.** `Refusal` has one reason, `UnknownPeer`, from
  `acknowledge` for an id outside the membership. Every `Err` leaves the rumor
  bit-for-bit as it was; `Rumor` is `Clone + PartialEq + Eq` so a caller can
  check.
- **Queries.** `status(peer) -> Option<Status>` (`None` for a non-member),
  `delivered()`, `exhausted()`, `peers()` (the caller's order),
  `attempt_limit()`.
- **No time.** The core reads no clock and holds no instant or span; pacing
  between rounds belongs to the caller.

The Ruby gem mirrors `Status` as a `Data` of the same name and fields and takes
`Rumor.new(peers, attempt_limit)` positionally; peer ids are Integers.

Initial, unstable API.
