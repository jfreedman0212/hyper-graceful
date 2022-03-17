use crate::connection::GracefulConnection;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Instant, Timeout};

/// Based on the [Tokio Shutdown Guide](https://tokio.rs/tokio/topics/shutdown)
/// This uses a broadcast channel to send a signal that the connections should begin shutting down
/// And then uses an mpsc channel, which waits for all the connections to complete
pub struct ConnectionManager {
    graceful_broadcast_tx: broadcast::Sender<()>,
    cancel_broadcast_tx: broadcast::Sender<()>,
    tracker_tx: mpsc::Sender<()>,
    tracker_rx: mpsc::Receiver<()>,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        let (graceful_broadcast_tx, _) = broadcast::channel::<()>(1);
        let (cancel_broadcast_tx, _) = broadcast::channel::<()>(1);
        let (tracker_tx, tracker_rx) = mpsc::channel(1);
        Self {
            graceful_broadcast_tx,
            cancel_broadcast_tx,
            tracker_tx,
            tracker_rx,
        }
    }
}

impl ConnectionManager {
    /// Manages a connection
    /// Note: You must still spawn / poll the connection future yourself!
    #[must_use]
    pub fn manage_connection<C, F, O>(
        &self,
        connection: C,
        on_shutdown: F,
    ) -> GracefulConnection<C, F>
    where
        C: Future<Output = O> + Send + 'static,
        F: Fn(Pin<&mut C>) + Send + 'static,
        O: Send + 'static,
    {
        let graceful_rx = self.graceful_broadcast_tx.subscribe();
        let cancel_rx = self.cancel_broadcast_tx.subscribe();
        GracefulConnection::new(
            connection,
            graceful_rx,
            on_shutdown,
            cancel_rx,
            self.tracker_tx.clone(),
        )
    }

    async fn graceful_shutdown_impl<X: MakeTimeout>(mut self, x: X) -> (usize, usize) {
        let graceful = self.graceful_broadcast_tx.send(()).unwrap_or(0);
        drop(self.tracker_tx);
        if x.make_timeout(self.tracker_rx.recv()).await.is_ok() {
            return (graceful, 0);
        }
        let forced = self.cancel_broadcast_tx.send(()).unwrap_or(0);
        self.tracker_rx.recv().await;
        (graceful - forced, forced)
    }

    /// Waits for the remaining connections to finish
    /// If the timeout is exceeded then they are forcefully closed
    /// Returns count of gracefully and forcibly closed connections
    pub async fn graceful_shutdown(self, timeout: Duration) -> (usize, usize) {
        self.graceful_shutdown_impl(timeout).await
    }

    /// Waits for the remaining connections to finish
    /// If the timeout is exceeded then they are forcefully closed
    /// Returns count of gracefully and forcibly closed connections
    pub async fn graceful_shutdown_by(self, instant: Instant) -> (usize, usize) {
        self.graceful_shutdown_impl(instant).await
    }
}

trait MakeTimeout {
    fn make_timeout<O: Future>(self, future: O) -> Timeout<O>;
}

impl MakeTimeout for Duration {
    fn make_timeout<O: Future>(self, future: O) -> Timeout<O> {
        tokio::time::timeout(self, future)
    }
}

impl MakeTimeout for Instant {
    fn make_timeout<O: Future>(self, future: O) -> Timeout<O> {
        tokio::time::timeout_at(self, future)
    }
}
