use super::{App, Application, Bundle, Environment, Kernel, KernelError, KernelPhase};

struct NoopBundle(&'static str);

impl Bundle for NoopBundle {
    fn name(&self) -> &'static str {
        self.0
    }
}

struct RecordingBundle {
    name: &'static str,
    events: std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>,
}

impl Bundle for RecordingBundle {
    fn name(&self) -> &'static str {
        self.name
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
            events: events.clone(),
        })
        .unwrap();
    kernel
        .register_bundle(RecordingBundle {
            name: "second",
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
