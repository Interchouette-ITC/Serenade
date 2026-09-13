//! Form builder, bind, and validation.

use std::collections::HashMap;
use std::sync::Arc;

use serenade_http::{Method, Request};
use serenade_security::{CSRF_FIELD_NAME, CsrfToken, CsrfTokenManager, SecurityError};
use serenade_validator::{Constraint, ConstraintViolationList, RecursiveValidator, Validator};

use crate::parse::parse_urlencoded;
use crate::render::RenderedForm;
use crate::{FormError, escape_attr, escape_html};

/// Outcome after [`Form::handle_request`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormStatus {
    /// Request was not a form submit for this form.
    NotSubmitted,
    /// Bound successfully (CSRF OK when enabled).
    Bound,
}

/// One named form field with optional constraints and current string value.
pub struct Field {
    name: String,
    value: String,
    constraints: Vec<Arc<dyn Constraint>>,
}

impl Field {
    /// Field name (HTML `name` attribute).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Current value (empty before bind / default).
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// HTML form with optional CSRF (on by default) and validator bridge.
pub struct Form {
    name: String,
    method: Method,
    action: String,
    fields: Vec<Field>,
    csrf_enabled: bool,
    csrf_token: Option<CsrfToken>,
    submitted: bool,
    violations: ConstraintViolationList,
}

impl Form {
    /// Starts a builder named `name` (also used as CSRF intention id).
    #[must_use]
    pub fn builder(name: impl Into<String>) -> FormBuilder {
        FormBuilder::new(name)
    }

    /// Form name / CSRF intention id.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the last [`Self::handle_request`] bound a submit.
    #[must_use]
    pub const fn is_submitted(&self) -> bool {
        self.submitted
    }

    /// Field values after bind (excludes CSRF field).
    #[must_use]
    pub fn data(&self) -> HashMap<String, String> {
        self.fields
            .iter()
            .map(|f| (f.name.clone(), f.value.clone()))
            .collect()
    }

    /// Returns a field value when present.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .map(Field::value)
    }

    /// Sets a field value when the field exists (for example edit prefill).
    ///
    /// Returns `true` when `name` matched a field.
    pub fn set(&mut self, name: &str, value: impl Into<String>) -> bool {
        if let Some(field) = self.fields.iter_mut().find(|f| f.name == name) {
            field.value = value.into();
            true
        } else {
            false
        }
    }

    /// Constraint violations from the last [`Self::validate`] call.
    #[must_use]
    pub const fn violations(&self) -> &ConstraintViolationList {
        &self.violations
    }

    /// Issues a CSRF token when CSRF is enabled (call before render on GET).
    ///
    /// # Errors
    ///
    /// Returns [`FormError::Csrf`] when token generation fails.
    pub fn prepare_csrf(&mut self, manager: &dyn CsrfTokenManager) -> Result<(), FormError> {
        if !self.csrf_enabled {
            return Ok(());
        }
        self.csrf_token = Some(manager.get_token(&self.name)?);
        Ok(())
    }

    /// Binds POST/PUT/PATCH `application/x-www-form-urlencoded` body and checks CSRF.
    ///
    /// # Errors
    ///
    /// Returns parse or CSRF errors. Wrong method yields [`FormStatus::NotSubmitted`] without error.
    pub fn handle_request(
        &mut self,
        request: &Request,
        manager: &dyn CsrfTokenManager,
    ) -> Result<FormStatus, FormError> {
        self.submitted = false;
        self.violations = ConstraintViolationList::new();
        if !matches!(request.method(), Method::Post | Method::Put | Method::Patch) {
            return Ok(FormStatus::NotSubmitted);
        }
        let values = parse_urlencoded(request.body())?;
        if self.csrf_enabled {
            let submitted = values.get(CSRF_FIELD_NAME).map_or("", String::as_str);
            let token = CsrfToken::new(&self.name, submitted);
            if !manager.is_token_valid(&token) {
                return Err(SecurityError::InvalidCsrfToken.into());
            }
            self.csrf_token = Some(token);
        }
        for field in &mut self.fields {
            if let Some(value) = values.get(&field.name) {
                field.value = value.clone();
            } else {
                field.value.clear();
            }
        }
        self.submitted = true;
        Ok(FormStatus::Bound)
    }

    /// Runs field constraints through `validator` (defaults to [`RecursiveValidator`] when omitted via [`Self::is_valid`]).
    pub fn validate(&mut self, validator: &dyn Validator) -> bool {
        let mut violations = ConstraintViolationList::new();
        for field in &self.fields {
            let refs: Vec<&dyn Constraint> = field.constraints.iter().map(AsRef::as_ref).collect();
            let field_violations = validator.validate_value(&field.value, &field.name, &refs);
            for v in field_violations.as_slice() {
                violations.add(v.clone());
            }
        }
        let ok = violations.is_empty();
        self.violations = violations;
        ok
    }

    /// Validates with [`RecursiveValidator`].
    #[must_use]
    pub fn is_valid(&mut self) -> bool {
        self.validate(&RecursiveValidator::new())
    }

    /// Renders a full HTML form (escaped values, CSRF hidden field when enabled).
    ///
    /// # Errors
    ///
    /// Returns [`FormError::Csrf`] when CSRF is enabled but no token was prepared or bound.
    pub fn render(&self) -> Result<RenderedForm, FormError> {
        let csrf_html = if self.csrf_enabled {
            let token = self
                .csrf_token
                .as_ref()
                .ok_or(SecurityError::InvalidCsrfToken)?;
            format!(
                r#"<input type="hidden" name="{}" value="{}" />"#,
                escape_attr(CSRF_FIELD_NAME),
                escape_attr(token.value())
            )
        } else {
            String::new()
        };
        let mut fields_html = String::new();
        for field in &self.fields {
            use std::fmt::Write as _;
            let _ = write!(
                fields_html,
                r#"<label for="{id}">{label}</label><input type="text" id="{id}" name="{name}" value="{value}" />"#,
                id = escape_attr(field.name()),
                label = escape_html(field.name()),
                name = escape_attr(field.name()),
                value = escape_attr(field.value()),
            );
        }
        let html = format!(
            r#"<form name="{name}" method="{method}" action="{action}">{csrf}{fields}</form>"#,
            name = escape_attr(&self.name),
            method = escape_attr(self.method.as_str()),
            action = escape_attr(&self.action),
            csrf = csrf_html,
            fields = fields_html,
        );
        Ok(RenderedForm { html })
    }
}

/// Builds a [`Form`].
pub struct FormBuilder {
    name: String,
    method: Method,
    action: String,
    fields: Vec<Field>,
    csrf_enabled: bool,
}

impl FormBuilder {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            method: Method::Post,
            action: String::new(),
            fields: Vec::new(),
            csrf_enabled: true,
        }
    }

    /// Sets the HTML form method (default POST).
    #[must_use]
    pub const fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    /// Sets the form `action` URL (empty = current path).
    #[must_use]
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = action.into();
        self
    }

    /// Enables or disables CSRF (default **true**).
    #[must_use]
    pub const fn csrf(mut self, enabled: bool) -> Self {
        self.csrf_enabled = enabled;
        self
    }

    /// Adds a text field with optional constraints.
    #[must_use]
    pub fn field(mut self, name: impl Into<String>, constraints: Vec<Arc<dyn Constraint>>) -> Self {
        self.fields.push(Field {
            name: name.into(),
            value: String::new(),
            constraints,
        });
        self
    }

    /// Finishes the builder.
    #[must_use]
    pub fn build(self) -> Form {
        Form {
            name: self.name,
            method: self.method,
            action: self.action,
            fields: self.fields,
            csrf_enabled: self.csrf_enabled,
            csrf_token: None,
            submitted: false,
            violations: ConstraintViolationList::new(),
        }
    }
}
