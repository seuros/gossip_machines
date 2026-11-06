# gossip-machines

> **AMS Demers**, NCC-4210, *Demers*-class relay vessel.

Engine-room flight manual: plan bounded rumor relays across neighboring ships.

Allocation-free `no_std` planner for one rumor under fixed unique peer IDs.
Each round selects unacknowledged peers up to fanout, preferring fewer previous
sends and then placement scores. Supply a changing random seed for randomized
peer choice. Send attempts are bounded per peer.

Planning charges an attempt, not successful delivery. Ack state and attempt
exhaustion are distinct. Empty memberships are already acknowledged. The caller
authenticates and correlates acks to the rumor/membership epoch. No discovery,
failure detection, SWIM, anti-entropy reconciliation, transport, or consensus.
An exhausted rumor may still be undelivered; it is not globally committed.
