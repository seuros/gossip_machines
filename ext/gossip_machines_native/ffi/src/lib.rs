//! Thin bindings for a single bounded rumor; the core owns all planner decisions.

use std::cell::RefCell;
use std::num::NonZeroU32;

use gossip_machines::Rumor;
use magnus::scan_args::{get_kwargs, scan_args};
use magnus::typed_data::Obj;
use magnus::{
    Error, Integer, RArray, RHash, Ruby, TryConvert, Value, function, method, prelude::*,
};
use placement_machines::NodeId;

const MAX_PEERS: usize = 64;
const FANOUT: usize = 3;

#[derive(Clone, Copy)]
struct Number(u64);

impl TryConvert for Number {
    fn try_convert(value: Value) -> Result<Self, Error> {
        if Integer::from_value(value).is_none() {
            return Err(Error::new(
                Ruby::get_with(value).exception_type_error(),
                "peer IDs, seeds and limits must be Integers",
            ));
        }
        u64::try_convert(value).map(Self)
    }
}

#[magnus::wrap(class = "GossipMachines::Rumor", free_immediately, size)]
struct Wrapper(RefCell<Rumor<MAX_PEERS>>);

impl Wrapper {
    fn new(ruby: &Ruby, args: &[Value]) -> Result<Self, Error> {
        let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
        let (peers, limit) = get_kwargs::<_, (Value, Number), (), ()>(
            args.keywords,
            &["peers", "attempt_limit"],
            &[],
        )?
        .required;
        let peers = RArray::from_value(peers)
            .ok_or_else(|| Error::new(ruby.exception_type_error(), "peers must be an Array"))?;
        if peers.len() > MAX_PEERS {
            return Err(Error::new(
                ruby.exception_arg_error(),
                "at most 64 real peers are supported",
            ));
        }
        let peers = peers
            .into_iter()
            .map(|value| Number::try_convert(value).map(|id| NodeId(id.0)))
            .collect::<Result<Vec<_>, Error>>()?;
        let limit = u32::try_from(limit.0)
            .map_err(|_| Error::new(ruby.exception_range_error(), "attempt_limit exceeds u32"))?;
        let limit = NonZeroU32::new(limit).ok_or_else(|| {
            Error::new(ruby.exception_arg_error(), "attempt_limit must be positive")
        })?;
        let rumor = Rumor::from_slice(&peers, limit)
            .ok_or_else(|| Error::new(ruby.exception_arg_error(), "duplicate peer IDs"))?;
        Ok(Self(RefCell::new(rumor)))
    }

    fn round(rb_self: Obj<Self>, seed: Number) -> Result<Vec<Option<u64>>, Error> {
        rb_self.check_frozen()?;
        let targets = rb_self.0.borrow_mut().round::<FANOUT>(seed.0);
        Ok(targets
            .into_iter()
            .map(|peer| peer.map(|id| id.0))
            .collect())
    }

    fn acknowledge(rb_self: Obj<Self>, peer: Number) -> Result<bool, Error> {
        rb_self.check_frozen()?;
        Ok(rb_self.0.borrow_mut().acknowledge(NodeId(peer.0)))
    }

    fn peers(&self) -> Vec<u64> {
        self.0.borrow().peers().iter().map(|id| id.0).collect()
    }

    fn status(&self, peer: Number) -> Option<(u32, bool)> {
        self.0.borrow().status(NodeId(peer.0))
    }

    fn delivered(&self) -> bool {
        self.0.borrow().delivered()
    }

    fn exhausted(&self) -> bool {
        self.0.borrow().exhausted()
    }

    fn attempt_limit(&self) -> u32 {
        self.0.borrow().attempt_limit()
    }
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("GossipMachines")?;
    let class = module.define_class("Rumor", ruby.class_object())?;
    class.const_set("MAX_PEERS", MAX_PEERS)?;
    class.const_set("FANOUT", FANOUT)?;
    class.define_singleton_method("new", function!(Wrapper::new, -1))?;
    class.define_method("round", method!(Wrapper::round, 1))?;
    class.define_method("acknowledge", method!(Wrapper::acknowledge, 1))?;
    class.define_method("peers", method!(Wrapper::peers, 0))?;
    class.define_method("status", method!(Wrapper::status, 1))?;
    class.define_method("delivered?", method!(Wrapper::delivered, 0))?;
    class.define_method("exhausted?", method!(Wrapper::exhausted, 0))?;
    class.define_method("attempt_limit", method!(Wrapper::attempt_limit, 0))
}
