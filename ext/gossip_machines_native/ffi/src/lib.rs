//! Thin bindings for a single bounded rumor; the core owns all planner decisions.

mod sealed;

use std::cell::RefCell;
use std::num::NonZeroU32;

use gossip_machines::{Acknowledgement, NodeId, Refusal, Rumor};
use magnus::typed_data::Obj;
use magnus::value::{Qfalse, Qtrue};
use magnus::{
    Error, Integer, RArray, Ruby, Symbol, TryConvert, Value, function, method, prelude::*,
};

/// `GossipMachines::Status`, resolved at init.
static STATUS: sealed::Sealed = sealed::Sealed::new("Status");

/// The core's const-generic `N` this gem is compiled with: peers one rumor
/// can hold, exposed as `Rumor::CAPACITY` like the core's.
const N: usize = 64;

/// The `RangeError` message for an `attempt_limit` outside `NonZeroU32`.
const LIMIT: &str = "attempt_limit must be in 1..=4294967295";

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

/// Private hook behind `Status.new`: the core's `Status` fields, every
/// pair of which is valid. `TypeError` for a non-Integer count or a non-boolean flag,
/// `RangeError` for a count outside `u32`.
fn status(ruby: &Ruby, attempts: Number, acknowledged: Value) -> Result<(u32, bool), Error> {
    let attempts = narrow(ruby, attempts, "attempts must be in 0..=4294967295")?;
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

/// `value` as a `u32`, or `RangeError` with `message`.
fn narrow(ruby: &Ruby, value: Number, message: &'static str) -> Result<u32, Error> {
    u32::try_from(value.0).map_err(|_| Error::new(ruby.exception_range_error(), message))
}

#[magnus::wrap(class = "GossipMachines::Rumor", free_immediately, size)]
struct Wrapper(RefCell<Rumor<N>>);

impl Wrapper {
    /// `TypeError` for a wrong type; `RangeError` for what the core answers
    /// `None` to (more than `CAPACITY` peers, a repeated id) and for an
    /// `attempt_limit` outside `NonZeroU32`.
    fn new(ruby: &Ruby, peers: Value, attempt_limit: Number) -> Result<Self, Error> {
        let peers = RArray::from_value(peers)
            .ok_or_else(|| Error::new(ruby.exception_type_error(), "peers must be an Array"))?;
        let count = peers.len();
        let peers = peers
            .into_iter()
            .map(|value| Number::try_convert(value).map(|id| NodeId(id.0)))
            .collect::<Result<Vec<_>, Error>>()?;
        let attempt_limit = NonZeroU32::new(narrow(ruby, attempt_limit, LIMIT)?)
            .ok_or_else(|| Error::new(ruby.exception_range_error(), LIMIT))?;
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
    fn round(this: Obj<Self>, seed: Number, max: Number) -> Result<Vec<u64>, Error> {
        this.check_frozen()?;
        let room = usize::try_from(max.0).map_or(N, |max| max.min(N));
        let mut buffer = [NodeId::default(); N];
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

    /// A query: building the `Status` after reading mutates nothing.
    fn status(ruby: &Ruby, this: &Self, peer: Number) -> Result<Option<Value>, Error> {
        let Some(status) = this.0.borrow().status(NodeId(peer.0)) else {
            return Ok(None);
        };
        STATUS
            .build(ruby, (status.attempts, status.acknowledged))
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
    STATUS.resolve(ruby, module)?;
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
