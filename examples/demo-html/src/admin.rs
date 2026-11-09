use std::sync::Arc;

use serenade_form::{Form, FormStatus};
use serenade_http::{HttpError, Request, Response};
use serenade_notifier::{Notification, Transport};
use serenade_view::path;
use serenade_workflow::{
    DefinitionBuilder, ExpressionGuard, MemoryMarkingStore, Workflow, WorkflowError,
};

use crate::admin_crud;
use crate::forms::{build_admin_action_form, build_login_form, build_logout_form, not_blank};
use crate::html::{
    admin_login_page, admin_page_with_logout, html_response, redirect, redirect_with_cookie,
};
use crate::state::{ADMIN_COOKIE, AppState, is_admin, ui_for};

pub(super) fn handle_admin_get(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf, state.matcher.collection())?;
        return Ok(html_response(
            200,
            admin_login_page(
                &ui_for(state, request),
                state.matcher.collection(),
                &login,
                None,
            ),
        ));
    }
    render_admin(state, request, None)
}

pub(super) fn handle_admin_login(
    state: &AppState,
    request: &Request,
) -> Result<Response, HttpError> {
    let mut form = Form::builder("admin-login")
        .field("token", not_blank())
        .build();
    match form.handle_request(request, &state.csrf) {
        Ok(FormStatus::Bound) if form.is_valid() => {
            let token = form.get("token").unwrap_or("").trim();
            if token != state.admin_token {
                let login = build_login_form(&state.csrf, state.matcher.collection())?;
                return Ok(html_response(
                    401,
                    admin_login_page(
                        &ui_for(state, request),
                        state.matcher.collection(),
                        &login,
                        Some("Invalid admin token."),
                    ),
                ));
            }
            let cookie =
                format!("{ADMIN_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400");
            Ok(redirect_with_cookie("/admin", &cookie))
        }
        _ => {
            let login = build_login_form(&state.csrf, state.matcher.collection())?;
            Ok(html_response(
                400,
                admin_login_page(
                    &ui_for(state, request),
                    state.matcher.collection(),
                    &login,
                    Some("Login failed. Try again."),
                ),
            ))
        }
    }
}

pub(super) fn handle_admin_logout(state: &AppState, request: &Request) -> Response {
    let mut form = Form::builder("admin-logout").build();
    let _ = form.handle_request(request, &state.csrf);
    let cookie = format!("{ADMIN_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    redirect_with_cookie("/", &cookie)
}

fn render_admin(
    state: &AppState,
    request: &Request,
    notice: Option<&str>,
) -> Result<Response, HttpError> {
    let pending = state.store.pending_comments();
    let mut forms = Vec::new();
    for comment in &pending {
        let cid = comment.id.to_string();
        let approve_path = path(
            state.matcher.collection(),
            "admin_approve",
            &[("id", cid.as_str())],
        )?;
        let reject_path = path(
            state.matcher.collection(),
            "admin_reject",
            &[("id", cid.as_str())],
        )?;
        let approve = build_admin_action_form(
            &state.csrf,
            &format!("approve-{}", comment.id),
            &approve_path,
            "Approve",
            "btn btn-success btn-sm",
        )?;
        let reject = build_admin_action_form(
            &state.csrf,
            &format!("reject-{}", comment.id),
            &reject_path,
            "Reject",
            "btn btn-outline-danger btn-sm",
        )?;
        forms.push((comment.id, approve, reject));
    }
    let logout = build_logout_form(&state.csrf, state.matcher.collection())?;
    let categories_html =
        admin_crud::categories_panel_html(&state.store, state.matcher.collection())?;
    Ok(html_response(
        200,
        admin_page_with_logout(
            &ui_for(state, request),
            state.matcher.collection(),
            &pending,
            &forms,
            &logout,
            &categories_html,
            notice,
        ),
    ))
}

pub(super) fn handle_admin_moderation(
    state: &AppState,
    request: &Request,
    approve: bool,
) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf, state.matcher.collection())?;
        return Ok(html_response(
            401,
            admin_login_page(
                &ui_for(state, request),
                state.matcher.collection(),
                &login,
                Some("Sign in first."),
            ),
        ));
    }
    let id = request
        .attributes()
        .get::<String>("id")
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| HttpError::bad_request("bad comment id"))?;
    let form_name = if approve {
        format!("approve-{id}")
    } else {
        format!("reject-{id}")
    };
    let mut form = Form::builder(form_name).build();
    if form.handle_request(request, &state.csrf).is_err() {
        return Ok(redirect("/admin"));
    }
    if approve {
        try_approve_comment_workflow(&format!("comment-{id}"))
            .map_err(|err| HttpError::bad_request(format!("moderation workflow blocked: {err}")))?;
        let _ = state.store.approve_comment(id);
        let _ = state.notifier.send(&Notification::sms(
            "+10000000000",
            format!("MyFeed: comment {id} approved"),
        ));
    } else {
        let _ = state.store.remove_comment(id);
    }
    Ok(redirect("/admin"))
}

/// Dogfood: pending→approved via [`ExpressionGuard`] (blocks subject `guest`).
fn try_approve_comment_workflow(subject_id: &str) -> Result<(), WorkflowError> {
    let definition = DefinitionBuilder::new()
        .places(["pending", "approved"])
        .edge("approve", "pending", "approved")
        .build()?;
    let mut workflow = Workflow::new(
        "myfeed_comment",
        definition,
        Arc::new(MemoryMarkingStore::new()),
    );
    workflow.add_guard(
        "approve",
        Arc::new(ExpressionGuard::new(r#"!(subject_id == "guest")"#)),
    );
    if !workflow.can(subject_id, "approve") {
        return Err(WorkflowError::NotEnabled {
            transition: "approve".into(),
        });
    }
    workflow.apply(subject_id, "approve")?;
    Ok(())
}

#[cfg(test)]
mod workflow_tests {
    use super::try_approve_comment_workflow;

    #[test]
    fn expression_guard_allows_admin_subject() {
        try_approve_comment_workflow("comment-1").expect("admin subject");
    }

    #[test]
    fn expression_guard_blocks_guest_subject() {
        assert!(try_approve_comment_workflow("guest").is_err());
    }
}
