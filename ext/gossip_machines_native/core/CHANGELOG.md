# Changelog

## [0.5.0](https://github.com/seuros/gossip_machines/compare/gossip-machines-v0.4.0...gossip-machines-v0.5.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* build the binding on matryona-machines 0.2; positive attempt_limit, compiler-checked symbols; ship no test files

### Features

* build the binding on matryona-machines 0.2; positive attempt_limit, compiler-checked symbols; ship no test files ([d52be3b](https://github.com/seuros/gossip_machines/commit/d52be3b7527f8e5b81a866775862fe01958efbc3))

## [0.4.0](https://github.com/seuros/gossip_machines/compare/gossip-machines-v0.3.0...gossip-machines-v0.4.0) (2026-11-14)


### ⚠ BREAKING CHANGES

* validate Status through the core, seal it, expose Rumor::CAPACITY

### Features

* validate Status through the core, seal it, expose Rumor::CAPACITY ([9fb790c](https://github.com/seuros/gossip_machines/commit/9fb790c5d570a70738086bc60a5b92aea589d93e))

## [0.3.0](https://github.com/seuros/gossip_machines/compare/gossip-machines-v0.2.2...gossip-machines-v0.3.0) (2026-11-13)


### ⚠ BREAKING CHANGES

* converted to the fleet conventions. One exhaustive Refusal per crate, Result from every operation that can refuse, a refusal changes nothing, provable idempotent repeats are honoured, Ruby mirrors the core. No compatibility aliases. Requires placement-machines >= 0.3. round writes into a caller buffer; NodeId is re-exported.

### Features

* refusals, named acknowledgements and caller-sized rounds for Rumor ([e78b95e](https://github.com/seuros/gossip_machines/commit/e78b95e0d6bedf52d0f92df1d2d5b749cf9e1339))

## [0.2.2](https://github.com/seuros/gossip_machines/compare/gossip-machines-v0.2.1...gossip-machines-v0.2.2) (2026-11-06)


### Bug Fixes

* crates.io keywords and categories for the core ([74fe5f3](https://github.com/seuros/gossip_machines/commit/74fe5f33cc306f1cf0bab78b5ec91448aa2d12cb))

## [0.2.1](https://github.com/seuros/gossip_machines/compare/gossip-machines-v0.2.0...gossip-machines-v0.2.1) (2026-11-06)


### Bug Fixes

* keep test sources out of the packaged gem and crate ([928c127](https://github.com/seuros/gossip_machines/commit/928c1271dce8a11d7f4c0c582930dc9d34e26aaa))

## [0.2.0](https://github.com/seuros/gossip_machines/compare/gossip-machines-v0.1.0...gossip-machines-v0.2.0) (2026-11-06)


### Features

* initial extraction ([b652160](https://github.com/seuros/gossip_machines/commit/b6521605fb2936a31e3d891c5ae8a71414593d28))
