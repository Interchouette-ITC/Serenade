//! Bind and run Axum with a Serenade [`AsyncHttpKernel`].

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::response::Response as AxumResponse;
use axum::routing::Router;
use serenade_http::{AsyncHttpKernel, Response};
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::{dispatch, to_axum};

/// Max request body size accepted by [`router`] / [`listen`] (16 MiB).
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Builds the Axum router that forwards every request to `kernel` via async dispatch.
pub fn router(kernel: Arc<AsyncHttpKernel>) -> Router {
    Router::new().fallback(serenade_service).with_state(kernel)
}

/// Oneshot handle that stops a [`BoundServer`] started by [`bind_server`].
pub struct ShutdownHandle {
    tx: oneshot::Sender<()>,
}

impl ShutdownHandle {
    /// Signals graceful shutdown of the paired server.
    pub fn shutdown(self) {
        let _ = self.tx.send(());
    }
}

/// Bound Axum serve task (await with [`await_bound`], stop via [`ShutdownHandle`]).
pub struct BoundServer {
    addr: SocketAddr,
    join: JoinHandle<std::io::Result<()>>,
}

impl BoundServer {
    /// Socket address the server is listening on.
    #[must_use]
    pub const fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}

/// Binds `addr` and returns a [`BoundServer`] plus [`ShutdownHandle`] (does not block).
///
/// Prefer [`listen`] for the usual "run until cancel" path. Use this when the caller
/// needs a handle for graceful shutdown in tests.
///
/// # Errors
///
/// Propagates bind errors.
pub async fn bind_server(
    addr: impl ToSocketAddrs,
    kernel: AsyncHttpKernel,
) -> std::io::Result<(BoundServer, ShutdownHandle)> {
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    let app = router(Arc::new(kernel));
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let join = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
    });
    Ok((
        BoundServer {
            addr: local_addr,
            join,
        },
        ShutdownHandle { tx: shutdown_tx },
    ))
}

/// Awaits a server from [`bind_server`] until it stops.
///
/// # Errors
///
/// Propagates server IO errors.
pub async fn await_bound(server: BoundServer) -> std::io::Result<()> {
    match server.join.await {
        Ok(result) => result,
        Err(error) => Err(std::io::Error::other(error)),
    }
}

/// Binds `addr` and serves every request through `kernel`.
///
/// Prefer this over hand-rolling `axum::serve` in app skeletons.
/// Controllers may be async ([`AsyncHttpKernel`]); wrap sync handlers with
/// [`AsyncHttpKernel::from_sync`](serenade_http::AsyncHttpKernel::from_sync).
///
/// # Errors
///
/// Propagates bind and server IO errors.
pub async fn listen(addr: impl ToSocketAddrs, kernel: AsyncHttpKernel) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, router(Arc::new(kernel))).await
}

async fn serenade_service(
    State(kernel): State<Arc<AsyncHttpKernel>>,
    request: Request,
) -> AxumResponse {
    let (parts, body) = request.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, MAX_BODY_BYTES).await else {
        return to_axum(&Response::text(413, "payload too large"));
    };
    dispatch::dispatch_async(kernel.as_ref(), &parts, bytes).await
}
