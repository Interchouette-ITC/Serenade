//! Sync and async profiler middleware.

use std::sync::Arc;
use std::time::Instant;

use serenade_http::{
    AsyncMiddleware, AsyncNext, HttpError, Middleware, ROUTE_ATTRIBUTE, Request, RequestHandler,
    Response,
};

use crate::PROFILER_TOKEN_ATTRIBUTE;
use crate::config::ProfilerConfig;
use crate::data::ProfileData;
use crate::generate_token;
use crate::logs::{install_log_scope, with_profile_scope};
use crate::store::ProfileStore;
use crate::toolbar::inject_toolbar;

/// Sync [`Middleware`] that records a profile and injects the toolbar.
#[derive(Clone, Debug)]
pub struct ProfilerMiddleware {
    store: Arc<ProfileStore>,
    config: ProfilerConfig,
}

impl ProfilerMiddleware {
    /// Middleware bound to `store` and `config`.
    #[must_use]
    pub const fn new(store: Arc<ProfileStore>, config: ProfilerConfig) -> Self {
        Self { store, config }
    }
}

impl Middleware for ProfilerMiddleware {
    fn process(
        &self,
        request: &mut Request,
        next: &dyn RequestHandler,
    ) -> Result<Response, HttpError> {
        if !self.config.enabled {
            return next.handle(request);
        }
        let token = generate_token();
        let method = request.method().as_str().to_owned();
        let path = request.path().to_owned();
        request
            .attributes_mut()
            .insert(PROFILER_TOKEN_ATTRIBUTE, token.clone());
        self.store
            .ensure(ProfileData::new(token.clone(), method, path));

        let started = Instant::now();
        let result = with_profile_scope(&self.store, token.clone(), || next.handle(request));
        let duration = started.elapsed();
        if let Some(route) = request.attributes().get::<String>(ROUTE_ATTRIBUTE).cloned() {
            self.store.set_route(&token, route);
        }
        match result {
            Ok(response) => {
                self.store.finish(&token, response.status(), duration);
                Ok(inject_toolbar(response, &token, &self.config.path_prefix))
            }
            Err(error) => {
                self.store.finish(&token, error.status_code(), duration);
                Err(error)
            }
        }
    }
}

/// Async [`AsyncMiddleware`] variant for [`serenade_http::AsyncHttpKernel`].
#[derive(Clone, Debug)]
pub struct AsyncProfilerMiddleware {
    store: Arc<ProfileStore>,
    config: ProfilerConfig,
}

impl AsyncProfilerMiddleware {
    /// Middleware bound to `store` and `config`.
    #[must_use]
    pub const fn new(store: Arc<ProfileStore>, config: ProfilerConfig) -> Self {
        Self { store, config }
    }
}

impl AsyncMiddleware for AsyncProfilerMiddleware {
    fn process<'a>(
        &'a self,
        request: &'a mut Request,
        next: AsyncNext<'a>,
    ) -> serenade_http::BoxFuture<'a, Result<Response, HttpError>> {
        Box::pin(async move {
            if !self.config.enabled {
                return next.run(request).await;
            }
            let token = generate_token();
            let method = request.method().as_str().to_owned();
            let path = request.path().to_owned();
            request
                .attributes_mut()
                .insert(PROFILER_TOKEN_ATTRIBUTE, token.clone());
            self.store
                .ensure(ProfileData::new(token.clone(), method, path));

            let _guard = install_log_scope(Arc::clone(&self.store), token.clone());
            let started = Instant::now();
            let result = next.run(request).await;
            let duration = started.elapsed();
            if let Some(route) = request.attributes().get::<String>(ROUTE_ATTRIBUTE).cloned() {
                self.store.set_route(&token, route);
            }
            match result {
                Ok(response) => {
                    self.store.finish(&token, response.status(), duration);
                    Ok(inject_toolbar(response, &token, &self.config.path_prefix))
                }
                Err(error) => {
                    self.store.finish(&token, error.status_code(), duration);
                    Err(error)
                }
            }
        })
    }
}
