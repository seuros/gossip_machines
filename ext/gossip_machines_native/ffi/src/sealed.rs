//! The `Data` classes the binding builds, resolved once at init.
//!
//! The binding builds `Status` and never reads one back from Ruby, so no
//! forged value can reach the core through it.

use std::sync::OnceLock;

use magnus::{Error, RClass, RModule, Ruby, Value, prelude::*, value::Opaque};

/// One sealed class of the gem, looked up by name once.
pub struct Sealed {
    name: &'static str,
    class: OnceLock<Opaque<RClass>>,
}

impl Sealed {
    /// The class `name` under the gem module, still to be resolved.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            class: OnceLock::new(),
        }
    }

    /// Look the class up at init and keep it from the garbage collector.
    pub fn resolve(&self, ruby: &Ruby, module: RModule) -> Result<(), Error> {
        let class: RClass = module.const_get(self.name)?;
        ruby.gc_register_mark_object(class);
        self.class.get_or_init(|| class.into());
        Ok(())
    }

    /// The class resolved at init.
    pub fn class(&self, ruby: &Ruby) -> Result<RClass, Error> {
        self.class
            .get()
            .map(|class| ruby.get_inner(*class))
            .ok_or_else(|| {
                Error::new(
                    ruby.exception_runtime_error(),
                    format!("{} was not resolved at init", self.name),
                )
            })
    }

    /// A new value, through the class's own `new` (so through its check).
    pub fn build<A: magnus::ArgList>(&self, ruby: &Ruby, args: A) -> Result<Value, Error> {
        self.class(ruby)?.funcall("new", args)
    }
}
