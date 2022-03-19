use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};

pin_project! {
    pub struct GracefulConnection<T, F> {
        #[pin]
        inner: T,
        graceful_receiver: broadcast::Receiver<()>,
        on_shutdown: Option<F>,
        cancel_receiver: broadcast::Receiver<()>,
        data: mpsc::Sender<()>,
    }
}

impl<T, F> GracefulConnection<T, F>
where
    T: Future,
    F: FnOnce(Pin<&mut T>),
{
    pub(crate) fn new(
        future: T,
        graceful_receiver: broadcast::Receiver<()>,
        on_shutdown: F,
        cancel_receiver: broadcast::Receiver<()>,
        data: mpsc::Sender<()>,
    ) -> Self {
        Self {
            inner: future,
            graceful_receiver,
            on_shutdown: Some(on_shutdown),
            cancel_receiver,
            data,
        }
    }
}

#[derive(Debug, Clone, Error)]
#[error("Connection was forcefully closed")]
pub struct ForcefullyClosed;

impl<T, F, O> Future for GracefulConnection<T, F>
where
    T: Future<Output = O>,
    F: FnOnce(Pin<&mut T>),
{
    type Output = Result<O, ForcefullyClosed>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        if this.graceful_receiver.try_recv().is_ok() && this.on_shutdown.is_some() {
            (this.on_shutdown.take().unwrap())(this.inner.as_mut());
        }
        if this.cancel_receiver.try_recv().is_ok() {
            return Poll::Ready(Err(ForcefullyClosed));
        }
        this.inner.poll(cx).map(|r| Ok(r))
    }
}
