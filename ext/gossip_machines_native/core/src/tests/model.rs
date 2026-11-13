//! Random operation streams checked against a naive model.
use super::*;

struct Draws(u64);
impl Draws {
    const fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).unwrap()
    }
}

/// Naive model: one row per peer, a round is "sort the eligible rows, take
/// the front", so it shares no selection loop with the core.
struct Model {
    rows: Vec<(NodeId, u32, bool)>,
    limit: u32,
}
impl Model {
    fn eligible(&self) -> Vec<NodeId> {
        self.rows
            .iter()
            .filter(|(_, attempts, acknowledged)| !acknowledged && *attempts < self.limit)
            .map(|(peer, _, _)| *peer)
            .collect()
    }
    fn round(&mut self, seed: u64, fanout: usize) -> Vec<NodeId> {
        let mut ranked: Vec<(u32, u64, NodeId)> = self
            .rows
            .iter()
            .filter(|(_, attempts, acknowledged)| !acknowledged && *attempts < self.limit)
            .map(|(peer, attempts, _)| (*attempts, placement_machines::score(seed, *peer), *peer))
            .collect();
        ranked.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
        ranked.truncate(fanout);
        let chosen: Vec<NodeId> = ranked.into_iter().map(|(_, _, peer)| peer).collect();
        for row in &mut self.rows {
            row.1 += u32::from(chosen.contains(&row.0));
        }
        chosen
    }
}

/// Answers seen: empty rounds, duplicate acks, unknown-peer refusals.
fn run_model(seed: u64) -> [usize; 3] {
    const N: usize = 9;
    let mut draws = Draws(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let members = draws.below(N + 1);
    // Ids 0..members scrambled into a sparse range; id zero can be a member.
    let stride = draws.next() | 1;
    let peers: Vec<NodeId> = (0..members as u64)
        .map(|index| NodeId(index.wrapping_mul(stride)))
        .collect();
    let attempt_limit = limit(u32::try_from(draws.below(4)).unwrap() + 1);
    let mut rumor = Rumor::<N>::new(&peers, attempt_limit).expect("distinct peers");
    let mut model = Model {
        rows: peers.iter().map(|peer| (*peer, 0, false)).collect(),
        limit: attempt_limit.get(),
    };
    let mut tally = [0; 3];
    for _ in 0..200 {
        let snapshot = rumor.clone();
        if draws.below(3) == 0 {
            let fanout = draws.below(5);
            let round_seed = draws.next();
            let expected = model.round(round_seed, fanout);
            assert_eq!(round(&mut rumor, round_seed, fanout), expected);
            if expected.is_empty() {
                tally[0] += 1;
                assert_eq!(rumor, snapshot);
            }
        } else {
            // Mostly members, sometimes a stranger.
            let known = draws.below(4) != 0 && !peers.is_empty();
            let peer = if known {
                peers[draws.below(peers.len())]
            } else {
                NodeId(draws.next())
            };
            let row = model.rows.iter_mut().find(|row| row.0 == peer);
            let expected = match row {
                None => Err(Refusal::UnknownPeer),
                Some((_, _, true)) => Ok(Acknowledgement::Duplicate),
                Some(row) => {
                    row.2 = true;
                    Ok(Acknowledgement::Acknowledged)
                }
            };
            assert_eq!(rumor.acknowledge(peer), expected);
            match expected {
                Ok(Acknowledgement::Acknowledged) => assert_ne!(rumor, snapshot),
                Ok(Acknowledgement::Duplicate) => {
                    tally[1] += 1;
                    assert_eq!(rumor, snapshot);
                }
                Err(Refusal::UnknownPeer) => {
                    tally[2] += 1;
                    assert_eq!(rumor, snapshot);
                }
            }
        }
        for (peer, attempts, acknowledged) in &model.rows {
            assert_eq!(rumor.status(*peer), Some(status(*attempts, *acknowledged)));
            assert!(*attempts <= model.limit);
        }
        assert_eq!(rumor.exhausted(), model.eligible().is_empty());
        assert_eq!(rumor.delivered(), model.rows.iter().all(|row| row.2));
        assert_eq!(rumor.peers(), peers.as_slice());
    }
    tally
}

#[test]
fn randomized_runs_match_the_naive_model() {
    let mut seen = [0_usize; 3];
    for seed in 1..=300 {
        for (total, count) in seen.iter_mut().zip(run_model(seed)) {
            *total += count;
        }
    }
    // Every "nothing changed" answer was exercised and clone-compared.
    assert!(seen.iter().all(|count| *count > 100), "{seen:?}");
}
