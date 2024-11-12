use nix::sys::signal::Signal;
use std::io;
use tokio::signal::unix::{signal, SignalKind};
use tokio_stream::{wrappers::SignalStream, Stream, StreamExt, StreamMap};

fn nix_to_tokio(signal: &Signal) -> SignalKind {
    match signal {
        Signal::SIGINT => SignalKind::interrupt(),
        Signal::SIGTERM => SignalKind::terminate(),
        Signal::SIGHUP => SignalKind::hangup(),
        Signal::SIGQUIT => SignalKind::quit(),
        Signal::SIGUSR1 => SignalKind::user_defined1(),
        Signal::SIGUSR2 => SignalKind::user_defined2(),
        Signal::SIGWINCH => SignalKind::window_change(),
        _ => panic!("unsupported signal: {:?}", signal),
    }
}

// This is a list of signals that are meaningful to forward to the child
// process. This includes signals that are used to force a process to
// terminate, as well as signals that are used to communicate with the
// process.
//
// This list only includes signals that can be caught and handled by the
// application. Signals that cannot be caught, such as SIGKILL and SIGSTOP,
// are not included. If the wrapper is killed by a `SIGKILL` or `SIGSTOP`,
// the child process will receive a `SIGTERM` signal -- see `exit_with_parent`.
const CHILD_FORWARDABLE_SIGNALS: [Signal; 7] = [
    Signal::SIGUSR1,
    Signal::SIGUSR2,
    Signal::SIGWINCH,
    Signal::SIGINT,
    Signal::SIGTERM,
    Signal::SIGHUP,
    Signal::SIGQUIT,
];

// Returns whether a signal returned by a `signal_stream` represents an intent
// to terminate the process. While most signals have the default behaviour of
// terminating the process if unhandled, this function is used to check for
// signals that are sent with the expectation to cause the process to terminate.
//
// This is a subset of the signals in `CHILD_FORWARDABLE_SIGNALS` for which the default
// handling behaviour is to terminate the process, as described in:
// https://man7.org/linux/man-pages/man7/signal.7.html
//
// As such, it excludes the following:
// - `SIGUSR1` and `SIGUSR2`, which are used for custom communication with the process
// - `SIGWINCH`, which notifies the process of a terminal resize (and whose default
//   behaviour is to be ignored)
// - `SIGHUP`, which is sometimes used to trigger configuration refreshes
//
// The objective is to ensure that only signals which were sent with the explicit
// intent to terminate the child process cause this process to terminate.
pub fn has_terminating_intent(signal: &Signal) -> bool {
    matches!(signal, Signal::SIGINT | Signal::SIGTERM | Signal::SIGQUIT)
}

pub fn signal_stream() -> io::Result<impl Stream<Item = Signal>> {
    let mut signals = StreamMap::new();

    for nix_signal in &CHILD_FORWARDABLE_SIGNALS {
        signals.insert(
            *nix_signal,
            SignalStream::new(signal(nix_to_tokio(nix_signal))?),
        );
    }

    Ok(signals.map(|(signal, _)| signal))
}
