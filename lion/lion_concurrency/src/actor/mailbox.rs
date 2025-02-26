//! Actor mailbox implementation for message passing.
//!
//! Mailboxes are the communication channel between actors, allowing
//! asynchronous message passing with bounded capacity limits.

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use std::fmt;
use std::time::Duration;
use thiserror::Error;

/// Error when sending a message to a mailbox
#[derive(Error, Debug)]
pub enum MailboxError {
    /// The mailbox is full (bounded capacity reached)
    #[error("mailbox is full")]
    Full,
    /// The mailbox has been closed or the actor is stopped
    #[error("mailbox is closed")]
    Closed,
}

/// A typed message for an actor
pub trait Message: Send + 'static {
    /// The response type for this message
    type Response: Send + 'static;
}

/// A handle to an actor's mailbox
#[derive(Clone)]
pub struct Mailbox<M: Message> {
    // Channel for sending messages to the actor
    sender: Sender<M>,
    // Maximum capacity of the mailbox
    capacity: usize,
    // Actor identifier for debugging and monitoring
    actor_id: String,
}

impl<M: Message> Mailbox<M> {
    /// Create a new mailbox with the specified capacity
    pub fn new(capacity: usize, actor_id: impl Into<String>) -> (Self, Receiver<M>) {
        let (sender, receiver) = bounded(capacity);
        let mailbox = Self {
            sender,
            capacity,
            actor_id: actor_id.into(),
        };
        (mailbox, receiver)
    }

    /// Send a message to the actor
    pub fn send(&self, message: M) -> Result<(), MailboxError> {
        self.sender.send(message).map_err(|_| MailboxError::Closed)
    }

    /// Try to send a message without blocking
    pub fn try_send(&self, message: M) -> Result<(), MailboxError> {
        self.sender.try_send(message).map_err(|e| match e {
            TrySendError::Full(_) => MailboxError::Full,
            TrySendError::Disconnected(_) => MailboxError::Closed,
        })
    }

    /// Send a message with a timeout
    pub fn send_timeout(&self, message: M, timeout: Duration) -> Result<(), MailboxError> {
        self.sender
            .send_timeout(message, timeout)
            .map_err(|_| MailboxError::Full)
    }

    /// Get the current capacity of the mailbox
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the actor ID associated with this mailbox
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }
}

impl<M: Message> fmt::Debug for Mailbox<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mailbox")
            .field("actor_id", &self.actor_id)
            .field("capacity", &self.capacity)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMessage(u32);

    impl Message for TestMessage {
        type Response = u32;
    }

    #[test]
    fn test_mailbox_send_receive() {
        let (mailbox, receiver) = Mailbox::new(10, "test-actor");

        mailbox.send(TestMessage(42)).unwrap();
        let message = receiver.recv().unwrap();

        assert_eq!(message.0, 42);
    }

    #[test]
    fn test_mailbox_capacity() {
        let (mailbox, _receiver) = Mailbox::new(2, "test-actor");

        // Two messages should succeed
        assert!(mailbox.try_send(TestMessage(1)).is_ok());
        assert!(mailbox.try_send(TestMessage(2)).is_ok());

        // Third message should fail with Full error
        let result = mailbox.try_send(TestMessage(3));
        assert!(matches!(result, Err(MailboxError::Full)));
    }

    #[test]
    fn test_mailbox_closed() {
        let (mailbox, receiver) = Mailbox::new(10, "test-actor");

        // Drop the receiver to close the channel
        drop(receiver);

        // Sending should fail with Closed error
        let result = mailbox.send(TestMessage(1));
        assert!(matches!(result, Err(MailboxError::Closed)));
    }

    #[test]
    fn test_mailbox_timeout() {
        let (mailbox, _receiver) = Mailbox::new(1, "test-actor");

        // Fill the mailbox
        mailbox.send(TestMessage(1)).unwrap();

        // Sending with timeout should fail
        let result = mailbox.send_timeout(TestMessage(2), Duration::from_millis(10));
        assert!(matches!(result, Err(MailboxError::Full)));
    }
}
