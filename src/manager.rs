use crate::connection::GracefulConnection;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::{broadcast, mpsc};

/// Based on the [Tokio Shutdown Guide](https://tokio.rs/tokio/topics/shutdown)
/// This uses a broadcast channel to send a signal that the connections should begin shutting down
/// And then uses an mpsc channel, which waits for all the connections to complete
pub struct ConnectionManager {
    broadcast_tx: broadcast::Sender<()>,
    tracker_tx: mpsc::Sender<()>,
    tracker_rx: mpsc::Receiver<()>,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        let (broadcast_tx, _) = broadcast::channel::<()>(1);
        let (tracker_tx, tracker_rx) = mpsc::channel(1);
        Self {
            broadcast_tx,
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
        let rx = self.broadcast_tx.subscribe();
        GracefulConnection::new(connection, rx, on_shutdown, self.tracker_tx.clone())
    }

    /// Returns the number of connections that were signalled
    pub fn graceful_shutdown(&self) -> usize {
        self.broadcast_tx.send(()).unwrap_or(0)
    }

    /// Waits for the remaining connections to finish
    pub async fn wait_for_connections(mut self) {
        // We are waiting for all Senders are dropped, including this one!
        drop(self.tracker_tx);
        self.tracker_rx.recv().await;
    }
}
