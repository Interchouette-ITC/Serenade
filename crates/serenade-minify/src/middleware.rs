use serenade_http::{
    AsyncMiddleware, AsyncNext, BoxFuture, HttpError, Middleware, Request, RequestHandler, Response,
};

use crate::minify_html;

/// Minifies UTF-8 `text/html` response bodies after the handler has rendered them.
///
/// Compressed responses and responses marked `Cache-Control: no-transform` are
/// left unchanged. This middleware can be used by both synchronous and asynchronous
/// HTTP kernels.
#[derive(Clone, Copy, Debug, Default)]
pub struct HtmlMinifier;

impl Middleware for HtmlMinifier {
    fn process(
        &self,
        request: &mut Request,
        next: &dyn RequestHandler,
    ) -> Result<Response, HttpError> {
        next.handle(request).and_then(minify_response)
    }
}

impl AsyncMiddleware for HtmlMinifier {
    fn process<'a>(
        &'a self,
        request: &'a mut Request,
        next: AsyncNext<'a>,
    ) -> BoxFuture<'a, Result<Response, HttpError>> {
        Box::pin(async move { next.run(request).await.and_then(minify_response) })
    }
}

fn minify_response(response: Response) -> Result<Response, HttpError> {
    let headers = response.headers();
    let is_html = headers
        .get("content-type")
        .and_then(|value| value.split(';').next())
        .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("text/html"));
    let is_encoded = headers.get("content-encoding").is_some();
    let forbids_transform = headers.get("cache-control").is_some_and(|value| {
        value
            .split(',')
            .any(|directive| directive.trim().eq_ignore_ascii_case("no-transform"))
    });
    if !is_html || is_encoded || forbids_transform {
        return Ok(response);
    }

    let Some(body) = response.body_str() else {
        return Ok(response);
    };
    let minified = minify_html(body)
        .map_err(|error| HttpError::failed(format!("HTML minification failed: {error}")))?;
    let content_length_present = headers.get("content-length").is_some();
    let content_length = minified.len().to_string();
    let mut response = response.with_body(minified.into_bytes());
    if content_length_present {
        response
            .headers_mut()
            .insert("content-length", content_length);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serenade_http::{AsyncHttpKernel, HttpKernel, Method};

    fn html_response() -> Response {
        Response::new(200)
            .with_header("content-type", "text/html; charset=utf-8")
            .with_body(b"<p> hello </p>".as_slice())
    }

    #[test]
    fn middleware_minifies_html_and_updates_existing_content_length() {
        let mut kernel = HttpKernel::new(|_: &mut Request| {
            Ok(html_response().with_header("content-length", "999"))
        });
        kernel.push_middleware(HtmlMinifier);

        let response = kernel.handle(Request::new(Method::Get, "/"));
        assert_eq!(response.body_str(), Some("<p>hello</p>"));
        assert_eq!(
            response.headers().get("content-length"),
            Some(response.body().len().to_string().as_str())
        );
    }

    #[test]
    fn middleware_leaves_responses_outside_its_scope_unchanged() {
        let responses = [
            Response::text(200, "<p> hello </p>"),
            Response::new(200).with_body(b"<p> hello </p>".as_slice()),
            html_response().with_header("content-encoding", "gzip"),
            html_response().with_header("cache-control", "public, No-Transform"),
            html_response().with_body([0xff, 0xfe]),
        ];
        for response in responses {
            let expected = response.clone();
            assert_eq!(
                minify_response(response).expect("response is skipped"),
                expected
            );
        }
    }

    #[test]
    fn middleware_minifies_without_content_length() {
        let response = minify_response(html_response()).expect("HTML minifies");
        assert_eq!(response.body_str(), Some("<p>hello</p>"));
        assert_eq!(response.headers().get("content-length"), None);
    }

    #[test]
    fn middleware_propagates_minification_errors() {
        let response = html_response().with_body(b"<script>const = ;</script>".as_slice());
        assert!(minify_response(response).is_err());
    }

    #[tokio::test]
    async fn async_middleware_minifies_html_responses() {
        let mut kernel = AsyncHttpKernel::from_sync(|_: &mut Request| Ok(html_response()));
        kernel.push_middleware(HtmlMinifier);

        let response = kernel.handle(Request::new(Method::Get, "/")).await;
        assert_eq!(response.body_str(), Some("<p>hello</p>"));
    }
}
