//! Thin bindings for a single bounded rumor; the core owns all planner decisions.

use std::cell::RefCell;
use std::num::NonZeroU32;

use gossip_machines::{Acknowledgement, NodeId, Refusal, Rumor};
use magnus::typed_data::Obj;
use magnus::value::{Qfalse, Qtrue};
use magnus::{Error, Ruby, Symbol, Value, function, method, prelude::*};
use matryona_machines::ruby::{Class, Registry, Symbols, exact_array, integer, positive};

/// The `Data` class the binding builds, resolved once at init. The binding
/// never reads a `Status` back from Ruby, so no forged value can reach the
/// core through it.
struct Classes {
    status: Class,
}

static CLASSES: Registry<Classes> = Registry::new("gossip_machines");

/// The core's const-generic `N` this gem is compiled with: peers one rumor
/// can hold, exposed as `Rumor::CAPACITY` like the core's.
const N: usize = 64;

matryona_machines::symbols! {
    /// The core's refusals as Symbols: the snake case of each variant.
    Refusals for Refusal as "a refusal" {
        UnknownPeer => "unknown_peer",
    }
}

matryona_machines::symbols! {
    /// The honoured answers of `acknowledge` as Symbols.
    Acknowledgements for Acknowledgement as "an acknowledgement" {
        Acknowledged => "acknowledged",
        Duplicate => "duplicate",
    }
}

/// Private hook behind `Status.new`: the core's `Status` fields, every
/// pair of which is valid. `TypeError` for a non-Integer count or a non-boolean flag,
/// `RangeError` for a count outside `u32`.
fn status(ruby: &Ruby, attempts: Value, acknowledged: Value) -> Result<(u32, bool), Error> {
    let attempts = integer(attempts, "attempts")?;
    let acknowledged = if Qtrue::from_value(acknowledged).is_some() {
        true
    } else if Qfalse::from_value(acknowledged).is_some() {
        false
    } else {
        return Err(Error::new(
            ruby.exception_type_error(),
            "acknowledged must be true or false",
        ));
    };
    Ok((attempts, acknowledged))
}

#[magnus::wrap(class = "GossipMachines::Rumor", free_immediately, size)]
struct Wrapper(RefCell<Rumor<N>>);

impl Wrapper {
    /// `TypeError` for a wrong type; `RangeError` for what the core answers
    /// `None` to (more than `CAPACITY` peers, a repeated id) and for an
    /// `attempt_limit` outside `NonZeroU32`.
    fn new(ruby: &Ruby, peers: Value, attempt_limit: Value) -> Result<Self, Error> {
        let attempt_limit: NonZeroU32 = positive(attempt_limit, "attempt_limit")?;
        let peers = exact_array(ruby, peers, "peers must be an Array")?;
        let count = peers.len();
        let peers = peers
            .into_iter()
            .map(|value| integer(value, "peer").map(NodeId))
            .collect::<Result<Vec<_>, Error>>()?;
        let rumor = Rumor::new(&peers, attempt_limit).ok_or_else(|| {
            let reason = if count > N {
                format!("peers holds {count} ids, more than CAPACITY ({N})")
            } else {
                String::from("peers repeats an id")
            };
            Error::new(ruby.exception_range_error(), reason)
        })?;
        Ok(Self(RefCell::new(rumor)))
    }

    /// At most `max` peers, each charged one attempt; `[]` when none is eligible.
    ///
    /// Never loses work: the frozen check and both conversions happen before
    /// the core is called, the core writes into a Rust buffer, and what follows
    /// only turns those ids into Integers.
    fn round(this: Obj<Self>, seed: Value, max: Value) -> Result<Vec<u64>, Error> {
        this.check_frozen()?;
        let (seed, max): (u64, u64) = (integer(seed, "seed")?, integer(max, "max")?);
        let room = usize::try_from(max).map_or(N, |max| max.min(N));
        let mut buffer = [NodeId::default(); N];
        let written = this.0.borrow_mut().round(seed, &mut buffer[..room]);
        Ok(buffer.iter().take(written).map(|peer| peer.0).collect())
    }

    fn acknowledge(ruby: &Ruby, this: Obj<Self>, peer: Value) -> Result<Symbol, Error> {
        this.check_frozen()?;
        let peer = NodeId(integer(peer, "peer")?);
        let outcome = this.0.borrow_mut().acknowledge(peer);
        Ok(match outcome {
            Ok(acknowledgement) => Acknowledgements::symbol(ruby, acknowledgement),
            Err(refused) => Refusals::symbol(ruby, refused),
        })
    }

    fn peers(&self) -> Vec<u64> {
        self.0.borrow().peers().iter().map(|id| id.0).collect()
    }

    /// A query: building the `Status` after reading mutates nothing.
    fn status(ruby: &Ruby, this: &Self, peer: Value) -> Result<Option<Value>, Error> {
        let peer = NodeId(integer(peer, "peer")?);
        let class = CLASSES.get(ruby)?.status.get(ruby);
        let Some(status) = this.0.borrow().status(peer) else {
            return Ok(None);
        };
        class
            .funcall("new", (status.attempts, status.acknowledged))
            .map(Some)
    }

    fn delivered(&self) -> bool {
        self.0.borrow().delivered()
    }

    fn exhausted(&self) -> bool {
        self.0.borrow().exhausted()
    }

    fn attempt_limit(&self) -> u32 {
        self.0.borrow().attempt_limit().get()
    }

    fn capacity(&self) -> usize {
        self.0.borrow().capacity()
    }
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("GossipMachines")?;
    let class = module.define_class("Rumor", ruby.class_object())?;
    class.undef_default_alloc_func();
    let classes = Classes {
        status: Class::resolve(ruby, module, "Status")?,
    };
    CLASSES.set(ruby, classes)?;
    module.define_singleton_method("__status", function!(status, 2))?;
    class.const_set("CAPACITY", Rumor::<N>::CAPACITY)?;
    class.define_singleton_method("new", function!(Wrapper::new, 2))?;
    class.define_method("round", method!(Wrapper::round, 2))?;
    class.define_method("acknowledge", method!(Wrapper::acknowledge, 1))?;
    class.define_method("peers", method!(Wrapper::peers, 0))?;
    class.define_method("status", method!(Wrapper::status, 1))?;
    class.define_method("delivered?", method!(Wrapper::delivered, 0))?;
    class.define_method("exhausted?", method!(Wrapper::exhausted, 0))?;
    class.define_method("attempt_limit", method!(Wrapper::attempt_limit, 0))?;
    class.define_method("capacity", method!(Wrapper::capacity, 0))
}
