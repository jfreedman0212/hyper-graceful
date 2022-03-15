use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::{broadcast, mpsc};

pin_project! {
    pub struct GracefulConnection<T, F> {
        #[pin]
        inner: T,
        #[pin]
        receiver: broadcast::Receiver<()>,
        on_shutdown: Option<F>,
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
        receiver: broadcast::Receiver<()>,
        on_shutdown: F,
        data: mpsc::Sender<()>,
    ) -> Self {
        Self {
            inner: future,
            receiver,
            on_shutdown: Some(on_shutdown),
            data,
        }
    }
}

impl<T, F, O> Future for GracefulConnection<T, F>
where
    T: Future<Output = O>,
    F: FnOnce(Pin<&mut T>),
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        if this.on_shutdown.is_some() && this.receiver.try_recv().is_ok() {
            (this.on_shutdown.take().unwrap())(this.inner.as_mut());
        }
        this.inner.poll(cx)
    }
}
