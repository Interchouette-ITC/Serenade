//! Default URL paths for `OpenAPI` explorers and the JSON document.

/// Paths used when mounting explorer UIs and pointing them at the `OpenAPI` JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApiUiPaths {
    /// Browser path pattern for Swagger UI (Actix style, e.g. `/swagger-ui/{_:.*}`).
    pub swagger_ui: String,
    /// Browser path for Redoc.
    pub redoc: String,
    /// Browser path for `RapiDoc`.
    pub rapidoc: String,
    /// Browser path for Scalar.
    pub scalar: String,
    /// URL (absolute path) where the `OpenAPI` JSON document is served.
    pub openapi_json: String,
}

impl Default for OpenApiUiPaths {
    fn default() -> Self {
        Self {
            swagger_ui: "/swagger-ui/{_:.*}".into(),
            redoc: "/redoc".into(),
            rapidoc: "/rapidoc".into(),
            scalar: "/scalar".into(),
            openapi_json: "/openapi.json".into(),
        }
    }
}

impl OpenApiUiPaths {
    /// Builds default Serenade explorer paths.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides the `OpenAPI` JSON URL that explorers load.
    #[must_use]
    pub fn with_openapi_json(mut self, path: impl Into<String>) -> Self {
        self.openapi_json = path.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_conventions() {
        let paths = OpenApiUiPaths::new();
        assert_eq!(paths.openapi_json, "/openapi.json");
        assert_eq!(paths.redoc, "/redoc");
        assert_eq!(paths.rapidoc, "/rapidoc");
        assert_eq!(paths.scalar, "/scalar");
        assert!(paths.swagger_ui.contains("swagger-ui"));
    }
}
