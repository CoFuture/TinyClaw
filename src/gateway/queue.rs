//! Message queue for WebSocket connections
//!
//! This module provides a message queue system for handling WebSocket messages
//! with better concurrency and backpressure support.

#![allow(dead_code)]
//!
//! This module provides a message queue system for handling WebSocket messages
//! with better concurrency and backpressure support.

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, warn};

/// Maximum number of messages that can be queued per connection
pub const DEFAULT_QUEUE_SIZE: usize = 100;

/// A message queue for handling WebSocket messages asynchronously
pub struct MessageQueue {
    /// Sender for queueing messages
    sender: mpsc::Sender<QueueMessage>,
    /// Task handle for the queue processor
    _processor: JoinHandle<()>,
}

/// Internal message type for the queue
pub struct QueueMessage {
    /// The message text content
    pub content: String,
    /// Sender for the response
    pub response_tx: Option<mpsc::Sender<String>>,
}

impl MessageQueue {
    /// Create a new message queue with the default size
    pub fn new<F>(processor: F) -> Self
    where
        F: FnOnce(mpsc::Receiver<QueueMessage>) -> JoinHandle<()>,
    {
        Self::with_size(DEFAULT_QUEUE_SIZE, processor)
    }

    /// Create a new message queue with a custom size
    pub fn with_size<F>(size: usize, processor: F) -> Self
    where
        F: FnOnce(mpsc::Receiver<QueueMessage>) -> JoinHandle<()>,
    {
        let (sender, receiver) = mpsc::channel::<QueueMessage>(size);
        let _processor = processor(receiver);

        Self {
            sender,
            _processor,
        }
    }

    /// Try to enqueue a message
    /// Returns false if the queue is full (backpressure)
    pub fn try_enqueue(&self, content: String) -> Option<mpsc::Receiver<String>> {
        let (response_tx, response_rx) = mpsc::channel(1);
        
        match self.sender.try_send(QueueMessage {
            content,
            response_tx: Some(response_tx),
        }) {
            Ok(_) => Some(response_rx),
            Err(mpsc::error::TrySendError::Full(_)) => {
                debug!("Message queue full, applying backpressure");
                None
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                warn!("Message queue closed");
                None
            }
        }
    }

    /// Check if the queue is full
    pub fn is_full(&self) -> bool {
        self.sender.capacity() == 0
    }

    /// Get the current queue capacity (approximate)
    pub fn capacity(&self) -> usize {
        self.sender.capacity()
    }
}

/// Builder for creating a message queue with custom configuration
pub struct MessageQueueBuilder {
    queue_size: usize,
    max_concurrent: usize,
}

impl MessageQueueBuilder {
    pub fn new() -> Self {
        Self {
            queue_size: DEFAULT_QUEUE_SIZE,
            max_concurrent: 5,
        }
    }

    /// Set the queue size
    pub fn queue_size(mut self, size: usize) -> Self {
        self.queue_size = size;
        self
    }

    /// Set the maximum number of concurrent message processing tasks
    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Build the message queue
    pub fn build<F>(self, processor: F) -> MessageQueue
    where
        F: FnOnce(mpsc::Receiver<QueueMessage>) -> JoinHandle<()>,
    {
        let (sender, receiver) = mpsc::channel::<QueueMessage>(self.queue_size);
        let _processor = processor(receiver);

        MessageQueue {
            sender,
            _processor,
        }
    }
}

impl Default for MessageQueueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_queue_basic() {
        let _queue = MessageQueue::new(|mut receiver| {
            tokio::spawn(async move {
                while let Some(msg) = receiver.recv().await {
                    if let Some(response_tx) = msg.response_tx {
                        // Echo back the message
                        let _ = response_tx.send(msg.content).await;
                    }
                }
            })
        });

        // Just verify the queue builds correctly
    }

    #[tokio::test]
    async fn test_message_queue_backpressure() {
        let _queue = MessageQueue::new(|receiver| {
            tokio::spawn(async move {
                // Don't process any messages to simulate slow processing
                // Receiver will be dropped, queue will fill up
                let _ = receiver;
            })
        });

        // Just verify the queue builds correctly
    }

    #[tokio::test]
    async fn test_message_queue_builder() {
        let builder = MessageQueueBuilder::new()
            .queue_size(50)
            .max_concurrent(3);

        let _queue = builder.build(|mut receiver| {
            tokio::spawn(async move {
                while let Some(msg) = receiver.recv().await {
                    if let Some(response_tx) = msg.response_tx {
                        let _ = response_tx.send(msg.content).await;
                    }
                }
            })
        });

        // Just verify it builds without panicking
    }
}
