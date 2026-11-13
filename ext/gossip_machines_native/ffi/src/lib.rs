//! Thin bindings for a single bounded rumor; the core owns all planner decisions.

use std::cell::RefCell;
use std::num::NonZeroU32;

use gossip_machines::{Acknowledgement, NodeId, Refusal, Rumor};
use magnus::typed_data::Obj;
use magnus::{
    Error, Integer, RArray, Ruby, Symbol, TryConvert, Value, function, method, prelude::*,
};

/// Peers one rumor can hold: the core's `N`.
const MAX_PEERS: usize = 64;

#[derive(Clone, Copy)]
struct Number(u64);

impl TryConvert for Number {
    fn try_convert(value: Value) -> Result<Self, Error> {
        if Integer::from_value(value).is_none() {
            return Err(Error::new(
                Ruby::get_with(value).exception_type_error(),
                "expected an Integer (unsigned 64-bit)",
            ));
        }
        u64::try_convert(value).map(Self)
    }
}

/// Ruby symbol name of a refusal: the snake case of the core variant.
const fn refusal(refused: Refusal) -> &'static str {
    match refused {
        Refusal::UnknownPeer => "unknown_peer",
    }
}

#[magnus::wrap(class = "GossipMachines::Rumor", free_immediately, size)]
struct Wrapper(RefCell<Rumor<MAX_PEERS>>);

impl Wrapper {
    /// Raises for what the core answers `None` to (`ArgumentError`) and for an
    /// `attempt_limit` outside `NonZeroU32` (`RangeError`).
    fn new(ruby: &Ruby, peers: Value, attempt_limit: Number) -> Result<Self, Error> {
        let peers = RArray::from_value(peers)
            .ok_or_else(|| Error::new(ruby.exception_type_error(), "peers must be an Array"))?;
        if peers.len() > MAX_PEERS {
            return Err(Error::new(
                ruby.exception_arg_error(),
                "at most 64 peers are supported",
            ));
        }
        let peers = peers
            .into_iter()
            .map(|value| Number::try_convert(value).map(|id| NodeId(id.0)))
            .collect::<Result<Vec<_>, Error>>()?;
        let attempt_limit = u32::try_from(attempt_limit.0)
            .ok()
            .and_then(NonZeroU32::new)
            .ok_or_else(|| {
                Error::new(
                    ruby.exception_range_error(),
                    "attempt_limit must be in 1..=4294967295",
                )
            })?;
        let rumor = Rumor::new(&peers, attempt_limit)
            .ok_or_else(|| Error::new(ruby.exception_arg_error(), "duplicate peer IDs"))?;
        Ok(Self(RefCell::new(rumor)))
    }

    /// At most `max` peers, each charged one attempt; `[]` when none is eligible.
    fn round(this: Obj<Self>, seed: Number, max: Number) -> Result<Vec<u64>, Error> {
        this.check_frozen()?;
        let room = usize::try_from(max.0).map_or(MAX_PEERS, |max| max.min(MAX_PEERS));
        let mut buffer = [NodeId::default(); MAX_PEERS];
        let written = this.0.borrow_mut().round(seed.0, &mut buffer[..room]);
        Ok(buffer.iter().take(written).map(|peer| peer.0).collect())
    }

    fn acknowledge(ruby: &Ruby, this: Obj<Self>, peer: Number) -> Result<Symbol, Error> {
        this.check_frozen()?;
        let outcome = this.0.borrow_mut().acknowledge(NodeId(peer.0));
        Ok(ruby.to_symbol(match outcome {
            Ok(Acknowledgement::Acknowledged) => "acknowledged",
            Ok(Acknowledgement::Duplicate) => "duplicate",
            Err(refused) => refusal(refused),
        }))
    }

    fn peers(&self) -> Vec<u64> {
        self.0.borrow().peers().iter().map(|id| id.0).collect()
    }

    fn status(ruby: &Ruby, this: &Self, peer: Number) -> Result<Option<Value>, Error> {
        let Some(status) = this.0.borrow().status(NodeId(peer.0)) else {
            return Ok(None);
        };
        let class: Value = ruby.define_module("GossipMachines")?.const_get("Status")?;
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
    class.const_set("MAX_PEERS", MAX_PEERS)?;
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
