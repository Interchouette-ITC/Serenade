//! Canonical sample: `DemoBundle` + `DemoExtension`.
//!
//! Shows package config, a DI service, routes, a console command, and an event
//! subscriber. Copy this layout when authoring a new bundle (see `docs-dev/BUNDLES.md`).

mod bundle;

pub use bundle::{
    DEMO_GREETING_SERVICE, DemoBundle, DemoExtension, DemoReady, DemoReadySubscriber, HelloCommand,
    demo_container,
};
