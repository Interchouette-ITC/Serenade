use thiserror::Error;

/// HTML or inline script could not be minified.
#[derive(Debug, Error)]
pub enum MinifyError {
    /// The HTML minifier emitted invalid UTF-8.
    #[error("HTML minifier emitted invalid UTF-8: {0}")]
    InvalidHtmlOutput(#[from] std::string::FromUtf8Error),
    /// An inline JavaScript block did not parse.
    #[error("inline JavaScript could not be minified: {0}")]
    JavaScript(String),
}
