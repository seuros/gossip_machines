use gossip_machines::Rumor;
use magnus::{Error, Ruby, function, method, prelude::*};
use matryona_machines::ruby::Class;

use crate::{CLASSES, Classes, N, Wrapper, status};

/// Defines `GossipMachines::Rumor` when Ruby loads the extension.
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
