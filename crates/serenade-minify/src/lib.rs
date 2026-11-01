//! HTML and inline JavaScript minification for static builds and HTTP responses.

mod error;
mod html;
mod javascript;
mod middleware;

pub use error::MinifyError;
pub use html::minify_html;
pub use middleware::HtmlMinifier;
