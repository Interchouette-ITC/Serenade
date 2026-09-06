//! Async transport adapter for queue backends (product chooses the backend).

use std::any::Any;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use crate::{Message, MessengerError};

/// Type-erased message ready to cross a [`Transport`].
#[derive(Clone)]
pub struct Envelope {
    message_name: &'static str,
    body: Arc<dyn Any + Send + Sync>,
}

impl Envelope {
    /// Wraps a typed [`Message`] for transport.
    #[must_use]
    pub fn new<M: Message>(message: M) -> Self {
        Self {
            message_name: M::NAME,
            body: Arc::new(message),
        }
    }

    /// Stable message name from [`Message::NAME`].
    #[must_use]
    pub const fn message_name(&self) -> &'static str {
        self.message_name
    }

    /// Downcasts the payload to `T` when the type matches.
    #[must_use]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.body.downcast_ref::<T>()
    }
}

impl std::fmt::Debug for Envelope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Envelope")
            .field("message_name", &self.message_name)
            .finish_non_exhaustive()
    }
}

/// Async send adapter for queue / worker backends.
///
/// Object-safe: returns a boxed future so products can store `Arc<dyn Transport>`.
pub trait Transport: Send + Sync {
    /// Enqueues `envelope` for later handling by a worker.
    fn send(
        &self,
        envelope: Envelope,
    ) -> Pin<Box<dyn Future<Output = Result<(), MessengerError>> + Send + '_>>;
}

/// In-memory [`Transport`] for tests and local workers (not durable).
#[derive(Clone, Default)]
pub struct InMemoryTransport {
    queue: Arc<Mutex<VecDeque<Envelope>>>,
}

impl InMemoryTransport {
    /// Creates an empty in-memory queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of envelopes waiting.
    ///
    /// # Panics
    ///
    /// Panics when the queue mutex is poisoned.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.lock().expect("in-memory transport lock").len()
    }

    /// Returns `true` when the queue is empty.
    ///
    /// # Panics
    ///
    /// Panics when the queue mutex is poisoned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Pops the next envelope (FIFO), if any.
    ///
    /// # Panics
    ///
    /// Panics when the queue mutex is poisoned.
    #[must_use]
    pub fn pop(&self) -> Option<Envelope> {
        self.queue
            .lock()
            .expect("in-memory transport lock")
            .pop_front()
    }
}

impl Transport for InMemoryTransport {
    /// Enqueues `envelope` at the back of the in-memory queue.
    ///
    /// # Panics
    ///
    /// Panics when the queue mutex is poisoned.
    fn send(
        &self,
        envelope: Envelope,
    ) -> Pin<Box<dyn Future<Output = Result<(), MessengerError>> + Send + '_>> {
        let queue = Arc::clone(&self.queue);
        Box::pin(async move {
            queue
                .lock()
                .expect("in-memory transport lock")
                .push_back(envelope);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Envelope, InMemoryTransport, Transport};
    use crate::{Command, Message, MessengerError};

    #[derive(Debug)]
    struct DemoCommand {
        id: u32,
    }

    impl Message for DemoCommand {
        const NAME: &'static str = "demo.command";
    }

    impl Command for DemoCommand {}

    struct FailingTransport;

    impl Transport for FailingTransport {
        fn send(
            &self,
            _envelope: Envelope,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<(), MessengerError>> + Send + '_>,
        > {
            Box::pin(async {
                Err(MessengerError::Transport {
                    message: "backend down".to_owned(),
                })
            })
        }
    }

    #[tokio::test]
    async fn in_memory_transport_queues_fifo() {
        let transport = InMemoryTransport::new();
        assert!(transport.is_empty());
        transport
            .send(Envelope::new(DemoCommand { id: 1 }))
            .await
            .expect("send 1");
        transport
            .send(Envelope::new(DemoCommand { id: 2 }))
            .await
            .expect("send 2");
        assert_eq!(transport.len(), 2);
        let first = transport.pop().expect("first");
        assert_eq!(first.message_name(), "demo.command");
        assert_eq!(first.downcast_ref::<DemoCommand>().expect("type").id, 1);
        assert!(first.downcast_ref::<u32>().is_none());
        let second = transport.pop().expect("second");
        assert_eq!(second.downcast_ref::<DemoCommand>().expect("type").id, 2);
        assert!(transport.pop().is_none());
        assert!(transport.is_empty());
    }

    #[tokio::test]
    async fn dyn_transport_send_works() {
        let memory = InMemoryTransport::new();
        let transport: Arc<dyn Transport> = Arc::new(memory.clone());
        transport
            .send(Envelope::new(DemoCommand { id: 9 }))
            .await
            .expect("send");
        assert_eq!(memory.len(), 1);
        assert_eq!(
            memory
                .pop()
                .expect("queued")
                .downcast_ref::<DemoCommand>()
                .expect("type")
                .id,
            9
        );
    }

    #[tokio::test]
    async fn failing_transport_returns_transport_error() {
        let transport = FailingTransport;
        let err = transport
            .send(Envelope::new(DemoCommand { id: 1 }))
            .await
            .expect_err("fail");
        assert_eq!(
            err,
            MessengerError::Transport {
                message: "backend down".to_owned(),
            }
        );
    }

    #[test]
    fn envelope_debug_includes_message_name() {
        let envelope = Envelope::new(DemoCommand { id: 3 });
        let debug = format!("{envelope:?}");
        assert!(debug.contains("demo.command"));
    }
}
