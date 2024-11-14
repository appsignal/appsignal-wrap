use std::future::Future;

use ::log::debug;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub async fn maybe_recv<T>(receiver: &mut Option<UnboundedReceiver<T>>) -> Option<Option<T>> {
    match receiver {
        Some(receiver) => Some(receiver.recv().await),
        None => None,
    }
}

pub fn maybe_spawn_tee<T: Clone + Send + 'static>(
    receiver: Option<UnboundedReceiver<T>>,
) -> (Option<UnboundedReceiver<T>>, Option<UnboundedReceiver<T>>) {
    match receiver {
        Some(receiver) => {
            let (first_receiver, second_receiver) = spawn_tee(receiver);
            (Some(first_receiver), Some(second_receiver))
        }
        None => (None, None),
    }
}

// An utility function that takes an unbounded receiver and returns two
// unbounded receivers that will receive the same items, spawning a task
// to read from the given receiver and write to the returned receivers.
// The items must implement the `Clone` trait.
pub fn spawn_tee<T: Clone + Send + 'static>(
    receiver: UnboundedReceiver<T>,
) -> (UnboundedReceiver<T>, UnboundedReceiver<T>) {
    let (future, first_receiver, second_receiver) = tee(receiver);

    tokio::spawn(future);

    (first_receiver, second_receiver)
}

fn tee<T: Clone + Send + 'static>(
    receiver: UnboundedReceiver<T>,
) -> (
    impl Future<Output = ()>,
    UnboundedReceiver<T>,
    UnboundedReceiver<T>,
) {
    let (first_sender, first_receiver) = unbounded_channel();
    let (second_sender, second_receiver) = unbounded_channel();

    let future = tee_loop(receiver, first_sender, second_sender);

    (future, first_receiver, second_receiver)
}

async fn tee_loop<T: Clone + Send + 'static>(
    mut receiver: UnboundedReceiver<T>,
    first_sender: UnboundedSender<T>,
    second_sender: UnboundedSender<T>,
) {
    while let Some(item) = receiver.recv().await {
        if let Err(err) = first_sender.send(item.clone()) {
            debug!("error sending item to first receiver: {}", err);
            break;
        }

        if let Err(err) = second_sender.send(item) {
            debug!("error sending item to second receiver: {}", err);
            break;
        }
    }
}
