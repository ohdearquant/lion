use futures::{Stream, StreamExt};
use std::{convert::Infallible, time::Duration};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

/// Configuration for SSE keep-alive settings
#[derive(Debug, Clone)]
pub struct SseKeepAlive {
    /// Interval between keep-alive messages
    pub interval: Duration,
    /// Text to send as keep-alive message
    pub text: String,
}

impl Default for SseKeepAlive {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(15),
            text: "keep-alive".to_string(),
        }
    }
}

/// A Server-Sent Events stream with configurable keep-alive
#[derive(Debug)]
pub struct EventStream<T> {
    /// The underlying stream of events
    pub stream: Box<dyn Stream<Item = Result<T, Infallible>> + Send>,
    /// Keep-alive configuration
    pub keep_alive: Option<SseKeepAlive>,
}

impl<T> EventStream<T> {
    /// Create a new event stream from a broadcast receiver
    pub fn new(rx: broadcast::Receiver<T>) -> Self
    where
        T: Clone + Send + 'static,
    {
        let stream = BroadcastStream::new(rx).map(|msg| {
            Ok::<_, Infallible>(msg.unwrap_or_else(|e| panic!("Error receiving message: {}", e)))
        });

        Self {
            stream: Box::new(stream),
            keep_alive: Some(SseKeepAlive::default()),
        }
    }

    /// Set the keep-alive configuration
    pub fn keep_alive(mut self, keep_alive: Option<SseKeepAlive>) -> Self {
        self.keep_alive = keep_alive;
        self
    }

    /// Set the keep-alive interval
    pub fn keep_alive_interval(mut self, interval: Duration) -> Self {
        if let Some(keep_alive) = &mut self.keep_alive {
            keep_alive.interval = interval;
        } else {
            self.keep_alive = Some(SseKeepAlive {
                interval,
                ..Default::default()
            });
        }
        self
    }

    /// Set the keep-alive text
    pub fn keep_alive_text(mut self, text: impl Into<String>) -> Self {
        if let Some(keep_alive) = &mut self.keep_alive {
            keep_alive.text = text.into();
        } else {
            self.keep_alive = Some(SseKeepAlive {
                text: text.into(),
                ..Default::default()
            });
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_event_stream() {
        let (tx, rx) = broadcast::channel(100);
        let stream = EventStream::new(rx);

        assert!(stream.keep_alive.is_some());
        let keep_alive = stream.keep_alive.unwrap();
        assert_eq!(keep_alive.interval, Duration::from_secs(15));
        assert_eq!(keep_alive.text, "keep-alive");

        // Test keep-alive configuration
        let stream = stream
            .keep_alive_interval(Duration::from_secs(30))
            .keep_alive_text("ping");

        let keep_alive = stream.keep_alive.unwrap();
        assert_eq!(keep_alive.interval, Duration::from_secs(30));
        assert_eq!(keep_alive.text, "ping");

        // Test disabling keep-alive
        let stream = stream.keep_alive(None);
        assert!(stream.keep_alive.is_none());

        // Clean up
        drop(tx);
    }
}