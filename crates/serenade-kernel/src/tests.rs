use super::{App, Application, BundleInterface, Environment, Kernel, KernelError, KernelPhase};

struct NoopBundle(&'static str);

impl BundleInterface for NoopBundle {
    fn name(&self) -> &'static str {
        self.0
    }
}

struct RecordingBundle {
    name: &'static str,
    deps: &'static [&'static str],
    events: std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>,
}

impl BundleInterface for RecordingBundle {
    fn name(&self) -> &'static str {
        self.name
    }

    fn dependencies(&self) -> &'static [&'static str] {
        self.deps
    }

    fn build(&self) -> Result<(), KernelError> {
        self.push("build")
    }

    fn boot(&self) -> Result<(), KernelError> {
        self.push("boot")
    }

    fn shutdown(&self) -> Result<(), KernelError> {
        self.push("shutdown")
    }
}

impl RecordingBundle {
    fn push(&self, event: &'static str) -> Result<(), KernelError> {
        self.events
            .lock()
            .map_err(|_| KernelError::Bundle {
                bundle: self.name,
                phase: event,
                message: "lock poisoned".to_owned(),
            })?
            .push(event);
        Ok(())
    }
}

struct NamedDeps {
    name: &'static str,
    deps: &'static [&'static str],
}

impl BundleInterface for NamedDeps {
    fn name(&self) -> &'static str {
        self.name
    }

    fn dependencies(&self) -> &'static [&'static str] {
        self.deps
    }
}

#[test]
fn version_is_non_empty() {
    assert_ne!(super::version(), "");
}

#[test]
fn empty_app_boots_with_zero_bundles() {
    let mut app = App::new(Environment::Test);
    app.boot().expect("empty app should boot");
    assert_eq!(app.kernel().phase(), KernelPhase::Booted);
    assert_eq!(app.kernel().bundle_names(), Vec::<&str>::new());
    assert!(app.kernel().debug());
    app.shutdown().expect("empty app should shutdown");
    assert_eq!(app.kernel().phase(), KernelPhase::Shutdown);
}

#[test]
fn environment_debug_defaults() {
    assert!(Environment::Dev.is_debug());
    assert!(Environment::Test.is_debug());
    assert!(!Environment::Prod.is_debug());
    assert!(!Kernel::new(Environment::Prod).debug());
    assert!(Kernel::new(Environment::Prod).with_debug(true).debug());
}

#[test]
fn environment_from_name_parses_ascii_case() {
    assert_eq!(Environment::from_name("DEV").unwrap(), Environment::Dev);
    assert_eq!(Environment::from_name("Test").unwrap(), Environment::Test);
    assert_eq!(Environment::from_name("prod").unwrap(), Environment::Prod);
    assert!(matches!(
        Environment::from_name("staging"),
        Err(KernelError::UnknownEnvironment(name)) if name == "staging"
    ));
}

#[test]
fn bundles_run_build_boot_then_reverse_shutdown() {
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut kernel = Kernel::new(Environment::Test);
    kernel
        .register_bundle(RecordingBundle {
            name: "first",
            deps: &[],
            events: events.clone(),
        })
        .unwrap();
    kernel
        .register_bundle(RecordingBundle {
            name: "second",
            deps: &[],
            events: events.clone(),
        })
        .unwrap();
    kernel.boot().unwrap();
    kernel.shutdown().unwrap();
    let log = events.lock().expect("lock").clone();
    assert_eq!(
        log,
        ["build", "build", "boot", "boot", "shutdown", "shutdown"]
    );
    assert_eq!(kernel.bundle_names(), ["first", "second"]);
}

#[test]
fn dependent_bundle_boots_after_dependency_even_if_registered_first() {
    let mut kernel = Kernel::new(Environment::Test);
    kernel
        .register_bundle(NamedDeps {
            name: "app",
            deps: &["framework"],
        })
        .unwrap();
    kernel
        .register_bundle(NamedDeps {
            name: "framework",
            deps: &[],
        })
        .unwrap();
    assert_eq!(kernel.bundle_names(), ["app", "framework"]);
    kernel.boot().unwrap();
    assert_eq!(kernel.bundle_names(), ["framework", "app"]);
}

#[test]
fn unknown_bundle_dependency_fails_compile() {
    let mut kernel = Kernel::new(Environment::Test);
    kernel
        .register_bundle(NamedDeps {
            name: "app",
            deps: &["missing"],
        })
        .unwrap();
    let error = kernel.compile().unwrap_err();
    assert_eq!(
        error,
        KernelError::UnknownBundleDependency {
            bundle: "app",
            dependency: "missing",
        }
    );
}

#[test]
fn cyclic_bundle_dependency_fails_compile() {
    let mut kernel = Kernel::new(Environment::Test);
    kernel
        .register_bundle(NamedDeps {
            name: "a",
            deps: &["b"],
        })
        .unwrap();
    kernel
        .register_bundle(NamedDeps {
            name: "b",
            deps: &["a"],
        })
        .unwrap();
    let error = kernel.compile().unwrap_err();
    assert!(matches!(error, KernelError::CyclicBundleDependency(_)));
}

#[test]
fn duplicate_bundle_name_is_rejected() {
    let mut kernel = Kernel::new(Environment::Dev);
    kernel.register_bundle(NoopBundle("core")).unwrap();
    let error = kernel.register_bundle(NoopBundle("core")).unwrap_err();
    assert_eq!(error, KernelError::DuplicateBundle("core"));
}

#[test]
fn register_after_compile_is_rejected() {
    let mut kernel = Kernel::new(Environment::Test);
    kernel.compile().unwrap();
    let error = kernel.register_bundle(NoopBundle("late")).unwrap_err();
    assert!(matches!(
        error,
        KernelError::InvalidState {
            action: "register",
            state: KernelPhase::Compiled
        }
    ));
}

#[test]
fn boot_after_shutdown_is_rejected() {
    let mut kernel = Kernel::new(Environment::Test);
    kernel.boot().unwrap();
    kernel.shutdown().unwrap();
    let error = kernel.boot().unwrap_err();
    assert!(matches!(
        error,
        KernelError::InvalidState {
            action: "boot",
            state: KernelPhase::Shutdown
        }
    ));
}
