//! Message queue implementation for the message bus.

use lion_core::message::Message;
use std::collections::VecDeque;
use std::sync::Mutex;

/// A thread-safe queue for messages
pub struct MessageQueue {
    /// The internal queue
    queue: Mutex<VecDeque<Message>>,
    
    /// The maximum capacity of the queue
    max_capacity: usize,
}

impl MessageQueue {
    /// Create a new message queue
    pub fn new(max_capacity: usize) -> Self {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(max_capacity)),
            max_capacity,
        }
    }
    
    /// Push a message to the queue
    pub fn push(&self, message: Message) -> Result<(), crate::error::MessageBusError> {
        let mut queue = self.queue.lock().unwrap();
        
        // Check if the queue is full
        if queue.len() >= self.max_capacity {
            return Err(crate::error::MessageBusError::BusFull);
        }
        
        // Add the message
        queue.push_back(message);
        
        Ok(())
    }
    
    /// Pop a message from the queue
    pub fn pop(&self) -> Option<Message> {
        let mut queue = self.queue.lock().unwrap();
        queue.pop_front()
    }
    
    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        let queue = self.queue.lock().unwrap();
        queue.is_empty()
    }
    
    /// Get the number of messages in the queue
    pub fn len(&self) -> usize {
        let queue = self.queue.lock().unwrap();
        queue.len()
    }
    
    /// Clear the queue
    pub fn clear(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.clear();
    }
}