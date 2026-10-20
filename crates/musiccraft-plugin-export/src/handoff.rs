//! Hands data built off the audio thread (a loaded sample, a wavetable) to
//! the audio thread through a lock-free queue, and takes what it replaced
//! back through a second queue, to be freed off the audio thread. Like the
//! convolution reverb's room (`shared.rs`), for any kind of data.

/// The non-audio ends. The plugin keeps it behind a lock that the audio
/// thread never takes.
pub struct Sender<T> {
    to_audio: rtrb::Producer<T>,
    garbage: rtrb::Consumer<T>,
}

/// The audio thread's ends.
pub struct Receiver<T> {
    incoming: rtrb::Consumer<T>,
    garbage: rtrb::Producer<T>,
}

/// A hand-over for up to `capacity` waiting items (main thread).
#[must_use]
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let (to_audio, incoming) = rtrb::RingBuffer::new(capacity);
    let (garbage_in, garbage) = rtrb::RingBuffer::new(capacity);
    (
        Sender { to_audio, garbage },
        Receiver {
            incoming,
            garbage: garbage_in,
        },
    )
}

impl<T> Sender<T> {
    /// Frees what came back, then queues `item`; gives it back when the
    /// queue is full.
    pub fn send(&mut self, item: T) -> Result<(), T> {
        self.collect();
        self.to_audio
            .push(item)
            .map_err(|rtrb::PushError::Full(item)| item)
    }

    /// Frees everything the audio thread handed back.
    pub fn collect(&mut self) {
        while self.garbage.pop().is_ok() {}
    }

    /// Items waiting for the audio thread.
    #[must_use]
    pub fn waiting(&self) -> usize {
        self.to_audio.buffer().capacity() - self.to_audio.slots()
    }
}

impl<T> Receiver<T> {
    /// Takes in every waiting item with `take`, which leaves in it what it
    /// replaced, and hands that back. Allocation-free. Stops while the
    /// return queue is full, so nothing is ever dropped here.
    pub fn take(&mut self, mut take: impl FnMut(&mut T)) {
        while self.garbage.slots() > 0 {
            let Ok(mut item) = self.incoming.pop() else {
                return;
            };
            take(&mut item);
            // Room was checked above.
            let _ = self.garbage.push(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_arrive_in_order_and_come_back() {
        let (mut sender, mut receiver) = channel::<Box<u32>>(2);
        sender.send(Box::new(1)).unwrap();
        sender.send(Box::new(2)).unwrap();
        assert!(sender.send(Box::new(3)).is_err(), "full");
        assert_eq!(sender.waiting(), 2);
        let mut current = Box::new(0);
        receiver.take(|item| std::mem::swap(&mut current, item));
        assert_eq!(*current, 2);
        assert_eq!(sender.waiting(), 0);
        // The replaced values came back and are freed here.
        sender.collect();
        assert!(sender.send(Box::new(4)).is_ok());
    }
}
