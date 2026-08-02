use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::core::Element;

use super::super::{
    LayoutEngine,
    incremental::{IncrementalFault, set_incremental_fault},
};
use super::TransactionalLayoutError;

#[test]
fn legacy_wrapper_does_not_erase_incremental_cause_from_dual_failure() {
    let before = Element::root();
    let mut after = Element::root();
    let mut created = Element::box_element().with_key("created");
    created.add_child(Element::text("fails text flow"));
    after.add_child(created);
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(&before, None, 20, 4);
    engine.set_text_flow_policy(0, "…", 1);
    set_incremental_fault(IncrementalFault::CreateBox);

    let result = catch_unwind(AssertUnwindSafe(|| {
        engine.try_compute_element_incremental(&after, Some(&previous), 20, 4)
    }));

    assert!(
        result.is_err(),
        "legacy TextFlow surface must fail loudly on dual causes"
    );
}

#[test]
fn recovery_accessors_expose_both_retained_causes() {
    let before = Element::root();
    let mut after = Element::root();
    after.add_child(Element::box_element().with_key("created"));
    let mut engine = LayoutEngine::new();
    let (previous, _) = engine.compute_element_incremental(&before, None, 20, 4);
    set_incremental_fault(IncrementalFault::CreateBox);
    super::super::context_sync::set_layout_compute_fault();

    let error = engine
        .try_compute_element_incremental_transactional(&after, Some(&previous), 20, 4)
        .expect_err("candidate and rebuild faults are retained");

    assert!(matches!(
        error,
        TransactionalLayoutError::RecoveryFailed { .. }
    ));
    assert!(error.incremental_failure().is_some());
    assert!(error.rebuild_failure().is_some());
    assert!(std::error::Error::source(&error).is_some());
}
