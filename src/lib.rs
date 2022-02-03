#![deny(unsafe_code, clippy::all)]

#[allow(unsafe_code)]
mod inner;

use self::inner::InnerPtr;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct WaitGroup(InnerPtr);

#[derive(Clone)]
pub struct Working(InnerPtr);

pub struct WaitFuture(InnerPtr);

impl WaitGroup {
    #[inline]
    pub fn new() -> Self {
        Self(InnerPtr::new())
    }

    #[inline]
    pub fn working(&self) -> Working {
        Working(self.0.clone())
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.0.count()
    }

    #[inline]
    pub fn wait(self) -> WaitFuture {
        WaitFuture(self.0)
    }
}

impl Default for WaitGroup {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Working {
    #[inline]
    pub fn count(&self) -> usize {
        self.0.count()
    }
}

impl WaitFuture {
    #[inline]
    pub fn count(&self) -> usize {
        self.0.count()
    }
}

impl Future for WaitFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0.count() == 0 {
            return Poll::Ready(());
        }
        self.0.register_waker(cx.waker());
        if self.0.count() == 0 {
            return Poll::Ready(());
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::WaitGroup;

    #[test]
    fn simple() {
        let wg = WaitGroup::new();
        let working_vec = vec![wg.working(); 100];
        assert_eq!(wg.count(), 100);
        let future = wg.wait();
        drop(future);
        drop(working_vec);
    }

    #[tokio::test]
    async fn tokio_test() {
        use tokio::time::{sleep, Duration};

        let wg = WaitGroup::new();
        assert_eq!(wg.count(), 0);

        let n = 100;
        for _ in 0..n {
            let working = wg.working();
            tokio::spawn(async move {
                sleep(Duration::from_millis(100)).await;
                drop(working);
            });
        }

        assert_eq!(wg.count(), 100);
        wg.wait().await;
    }
}
