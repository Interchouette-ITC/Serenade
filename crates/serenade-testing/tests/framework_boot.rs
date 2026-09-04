//! Integration: `FrameworkBundle` boots under `SerenadeTestKernel`.

use serenade_bundle::FrameworkBundle;
use serenade_kernel::KernelPhase;
use serenade_testing::SerenadeTestKernel;

#[test]
fn framework_bundle_boots_under_test_kernel() {
    let mut app = SerenadeTestKernel::new();
    app.register_bundle(FrameworkBundle)
        .expect("register FrameworkBundle");
    app.boot().expect("boot");
    assert_eq!(app.kernel().phase(), KernelPhase::Booted);
    assert!(app.kernel().bundle_names().contains(&"framework"));
    app.shutdown().expect("shutdown");
}
