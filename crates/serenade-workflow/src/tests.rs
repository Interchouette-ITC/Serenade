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

#[test]
fn marking_empty_len_and_is_empty() {
    let empty = Marking::empty();
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
    let one = Marking::single("draft");
    assert_eq!(one.len(), 1);
    assert!(!one.is_empty());
}

#[test]
fn workflow_exposes_name_and_definition() {
    let wf = article_workflow();
    assert_eq!(wf.name(), "article");
    assert_eq!(wf.definition().places(), ["draft", "published", "archived"]);
    assert!(wf.definition().transition("publish").is_some());
    assert!(!wf.can("doc", "nope"));
}

#[test]
fn definition_builder_place_and_validation_errors() {
    let ok = DefinitionBuilder::new()
        .place("draft")
        .place("published")
        .edge("publish", "draft", "published")
        .build()
        .expect("ok");
    assert_eq!(ok.places(), ["draft", "published"]);
    assert_eq!(ok.initial(), &Marking::single("draft"));

    assert_eq!(
        DefinitionBuilder::new().build().expect_err("empty"),
        WorkflowError::EmptyPlaces
    );
    assert_eq!(
        DefinitionBuilder::new()
            .place("")
            .build()
            .expect_err("empty place"),
        WorkflowError::EmptyPlaceName
    );
    assert_eq!(
        DefinitionBuilder::new()
            .places(["a"])
            .transition(Transition::new("", ["a"], ["a"]))
            .build()
            .expect_err("empty transition"),
        WorkflowError::EmptyTransitionName
    );
    assert_eq!(
        DefinitionBuilder::new()
            .places(["a", "b"])
            .edge("go", "a", "b")
            .edge("go", "b", "a")
            .build()
            .expect_err("dup transition"),
        WorkflowError::DuplicateTransition {
            transition: "go".into(),
        }
    );
    assert_eq!(
        DefinitionBuilder::new()
            .places(["a"])
            .transition(Transition::new("noop", [] as [&str; 0], ["a"]))
            .build()
            .expect_err("empty from"),
        WorkflowError::EmptyPlaceName
    );
    assert_eq!(
        DefinitionBuilder::new()
            .places(["a"])
            .initial(Marking::single("missing"))
            .build()
            .expect_err("bad initial"),
        WorkflowError::UnknownInitialPlace {
            place: "missing".into(),
        }
    );
}

#[test]
fn star_guard_applies_to_all_transitions() {
    let mut wf = article_workflow();
    wf.add_guard(
        "*",
        Arc::new(|ctx: &crate::TransitionContext<'_>| Err(block(ctx.transition.name(), "frozen"))),
    );
    assert!(!wf.can("x", "publish"));
    assert!(!wf.can("x", "archive"));
}
