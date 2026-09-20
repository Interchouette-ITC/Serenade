use std::sync::{Arc, Mutex};

use crate::{
    DefinitionBuilder, Marking, MarkingStore, MemoryMarkingStore, Transition, TransitionListener,
    Workflow, WorkflowError, block,
};

fn article_workflow() -> Workflow {
    let definition = DefinitionBuilder::new()
        .places(["draft", "published", "archived"])
        .edge("publish", "draft", "published")
        .edge("archive", "published", "archived")
        .build()
        .expect("valid definition");
    Workflow::new("article", definition, Arc::new(MemoryMarkingStore::new()))
}

#[test]
fn applies_legal_transition_and_rejects_illegal() {
    let wf = article_workflow();
    assert_eq!(wf.marking("doc-1"), Marking::single("draft"));
    assert!(wf.can("doc-1", "publish"));
    assert!(!wf.can("doc-1", "archive"));

    let next = wf.apply("doc-1", "publish").expect("publish");
    assert_eq!(next, Marking::single("published"));
    assert_eq!(wf.marking("doc-1"), Marking::single("published"));

    let err = wf.apply("doc-1", "publish").expect_err("already published");
    assert_eq!(
        err,
        WorkflowError::NotEnabled {
            transition: "publish".into(),
        }
    );

    assert_eq!(wf.enabled_transitions("doc-1"), vec!["archive"]);
    wf.apply("doc-1", "archive").expect("archive");
    assert_eq!(wf.enabled_transitions("doc-1"), [] as [&str; 0]);
}

#[test]
fn unknown_transition_errors() {
    let wf = article_workflow();
    let err = wf.apply("x", "nope").expect_err("unknown");
    assert_eq!(
        err,
        WorkflowError::UnknownTransition {
            transition: "nope".into(),
        }
    );
}

#[test]
fn guard_blocks_transition() {
    let mut wf = article_workflow();
    wf.add_guard(
        "publish",
        Arc::new(|ctx: &crate::TransitionContext<'_>| {
            if ctx.subject_id == "blocked" {
                Err(block(ctx.transition.name(), "not allowed"))
            } else {
                Ok(())
            }
        }),
    );
    assert!(!wf.can("blocked", "publish"));
    assert!(wf.can("ok", "publish"));
    let err = wf.apply("blocked", "publish").expect_err("guard");
    assert!(matches!(err, WorkflowError::GuardBlocked { .. }));
}

#[test]
fn multi_place_transition() {
    let definition = DefinitionBuilder::new()
        .places(["a", "b", "c", "done"])
        .transition(Transition::new("join", ["a", "b"], ["done"]))
        .initial(Marking::from_places(["a", "b"]))
        .build()
        .expect("valid");
    let wf = Workflow::new("join", definition, Arc::new(MemoryMarkingStore::new()));
    assert!(wf.can("s", "join"));
    let next = wf.apply("s", "join").expect("join");
    assert_eq!(next, Marking::single("done"));
}

#[test]
fn definition_rejects_unknown_place_on_edge() {
    let err = DefinitionBuilder::new()
        .places(["draft"])
        .edge("publish", "draft", "published")
        .build()
        .expect_err("unknown to");
    assert!(matches!(err, WorkflowError::UnknownPlace { .. }));
}

#[test]
fn definition_rejects_duplicate_place() {
    let err = DefinitionBuilder::new()
        .places(["a", "a"])
        .build()
        .expect_err("dup");
    assert_eq!(err, WorkflowError::DuplicatePlace { place: "a".into() });
}

#[test]
fn listener_sees_completed_apply() {
    struct Rec(Mutex<Vec<String>>);
    impl TransitionListener for Rec {
        fn on_completed(&self, ctx: &crate::CompletedContext<'_>) {
            self.0.lock().expect("lock").push(format!(
                "{}:{}",
                ctx.subject_id,
                ctx.transition.name()
            ));
        }
    }
    let mut wf = article_workflow();
    let rec = Arc::new(Rec(Mutex::new(Vec::new())));
    wf.add_listener(rec.clone());
    wf.apply("n1", "publish").expect("publish");
    assert_eq!(rec.0.lock().expect("lock").as_slice(), ["n1:publish"]);
}

#[test]
fn version_is_nonzero() {
    assert_ne!(crate::version(), "");
}

#[test]
fn memory_store_clear_resets_to_initial() {
    let store = Arc::new(MemoryMarkingStore::new());
    let definition = DefinitionBuilder::new()
        .places(["draft", "published"])
        .edge("publish", "draft", "published")
        .build()
        .expect("ok");
    let wf = Workflow::new("a", definition, store.clone());
    wf.apply("s", "publish").expect("publish");
    store.clear("s");
    assert_eq!(wf.marking("s"), Marking::single("draft"));
}
