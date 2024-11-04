mod check_in;
mod cli;
mod log;

mod client;
mod exit;
mod ndjson;
mod package;
mod signals;
mod timestamp;

use crate::check_in::{CronKind, HeartbeatConfig};
use crate::cli::Cli;
use crate::client::client;
use crate::log::{LogConfig, LogMessage, LogSeverity};
use crate::package::NAME;
use crate::signals::{has_terminating_intent, reset_sigpipe, signal_stream};
use crate::timestamp::SystemTimestamp;

use ::log::{debug, error, trace};
use std::os::unix::process::ExitStatusExt;
use std::process::{exit, ExitStatus, Stdio};
use std::{
    io,
    io::{stderr, stdout, Write},
};
use timestamp::MonotonicTimestamp;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::{interval, Duration, MissedTickBehavior};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use clap::Parser;
use env_logger::Env;

fn main() {
    reset_sigpipe();

    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            let level = record.level().to_string().to_ascii_lowercase();
            writeln!(buf, "{}: {}: {}", NAME, level, record.args())
        })
        .init();

    let cli = Cli::parse();
    cli.warn();

    match start(cli) {
        Ok(code) => exit(code),
        Err(err) => error!("{}", err),
    }
}

fn command(argv: &[String], log: &LogConfig) -> Command {
    let mut command = Command::new(argv[0].clone());
    for arg in argv[1..].iter() {
        command.arg(arg);
    }

    if log.origin.is_out() {
        command.stdout(Stdio::piped());
    }

    if log.origin.is_err() {
        command.stderr(Stdio::piped());
    }

    unsafe {
        command.pre_exec(exit::exit_with_parent);
    }

    command
}

// Pipes lines from an asynchronous reader to a synchronous writer, returning
// a join handle for the task piping the lines, which must not be dropped for
// the lifetime of the process, and a channel receiver that will receive each
// line as it is written.
async fn pipe_lines(
    from: impl AsyncRead + Unpin + Send + 'static,
    mut to: impl Write + Send + 'static,
    sender: UnboundedSender<Option<String>>,
) {
    let mut from = BufReader::new(from).lines();

    loop {
        match from.next_line().await {
            Ok(Some(line)) => {
                if let Err(err) = writeln!(to, "{}", line) {
                    debug!("error writing line: {}", err);
                    break;
                }

                if let Err(err) = sender.send(Some(line)) {
                    debug!("error sending line: {}", err);
                    break;
                };
            }
            Ok(None) => break,
            Err(err) => {
                debug!("error reading line: {}", err);
                break;
            }
        }
    }

    if let Err(err) = sender.send(None) {
        debug!("error sending EOF: {}", err);
    }
}

async fn send_request(request: Result<reqwest::Request, reqwest::Error>) {
    let request = match request {
        Ok(request) => request,
        Err(err) => {
            debug!("error creating request: {}", err);
            return;
        }
    };

    match client().execute(request.try_clone().unwrap()).await {
        Ok(response) => {
            if !response.status().is_success() {
                debug!("request failed with status: {}", response.status());
            } else {
                trace!("request successful: {}", request.url());
            }
        }
        Err(err) => {
            debug!("error sending request: {:?}", err);
        }
    };
}

async fn heartbeat_loop(config: HeartbeatConfig, cancel: CancellationToken) {
    let mut interval = interval(Duration::from_secs(30));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    // Ensure at least one heartbeat is sent.
    send_request(config.request(&mut SystemTimestamp)).await;
    interval.tick().await;

    // After a heartbeat has been sent, cancel immediately on request, without
    // waiting for the next scheduled heartbeat.
    loop {
        select!(
            _ = cancel.cancelled() => break,
            _ = interval.tick() => send_request(config.request(&mut SystemTimestamp)).await,
        );
    }
}

async fn maybe_recv<T>(receiver: &mut Option<UnboundedReceiver<T>>) -> Option<T> {
    match receiver {
        Some(receiver) => receiver.recv().await,
        None => None,
    }
}

async fn log_loop(
    log: LogConfig,
    mut stdout: Option<UnboundedReceiver<Option<String>>>,
    mut stderr: Option<UnboundedReceiver<Option<String>>>,
) {
    let mut timestamp = MonotonicTimestamp::new(SystemTimestamp);

    if stdout.is_none() && stderr.is_none() {
        return;
    }

    let mut messages = Vec::new();
    let mut interval = interval(Duration::from_secs(10));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let tasks = TaskTracker::new();

    loop {
        if messages.len() >= 100 {
            let request = log.request(std::mem::take(&mut messages));
            tasks.spawn(send_request(request));
            interval.reset();
        }

        select! {
            Some(maybe_line) = maybe_recv(&mut stdout) => {
                match maybe_line {
                    None => {
                        stdout = None;
                        if stderr.is_none() {
                            break;
                        }
                    }
                    Some(line) => {
                        messages.push(LogMessage::new(&log, &mut timestamp, LogSeverity::Info, line));
                    }
                }
            }

            Some(maybe_line) = maybe_recv(&mut stderr) => {
                match maybe_line {
                    None => {
                        stderr = None;
                        if stdout.is_none() {
                            break;
                        }
                    }
                    Some(line) => {
                        messages.push(LogMessage::new(&log, &mut timestamp, LogSeverity::Error, line));
                    }
                }
            }

            _ = interval.tick() => {
                if stdout.is_none() && stderr.is_none() {
                    break;
                }

                if !messages.is_empty() {
                    let request = log.request(std::mem::take(&mut messages));
                    tasks.spawn(send_request(request));
                }
            }

            else => break
        }
    }

    if !messages.is_empty() {
        let request = log.request(messages);
        tasks.spawn(send_request(request));
    }

    tasks.close();
    tasks.wait().await;
}

async fn forward_signals_and_wait(mut child: Child) -> io::Result<ExitStatus> {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    let mut signals = signal_stream()?;

    loop {
        select! {
            biased;

            status = child.wait() => {
                return status
            }

            Some(signal) = signals.next() => {
                if let Some(id) = child.id() {
                    let pid = Pid::from_raw(id.try_into().expect("Invalid PID"));
                    match kill(pid, signal) {
                        Ok(_) => trace!("forwarded signal to child: {}", signal),
                        Err(err) => debug!("error forwarding signal to child: {}", err),
                    };
                } else {
                    debug!("cannot forward signal to child: child process has no PID");
                }
            }
        }
    }
}

#[tokio::main]
async fn start(cli: Cli) -> Result<i32, Box<dyn std::error::Error>> {
    let cron = cli.cron();
    let log = cli.log();

    let tasks = TaskTracker::new();

    let mut child = command(&cli.command, &log).spawn()?;

    let stdout = if log.origin.is_out() {
        let (sender, receiver) = unbounded_channel();
        tasks.spawn(pipe_lines(child.stdout.take().unwrap(), stdout(), sender));
        Some(receiver)
    } else {
        None
    };

    let stderr = if log.origin.is_err() {
        let (sender, receiver) = unbounded_channel();
        tasks.spawn(pipe_lines(child.stderr.take().unwrap(), stderr(), sender));
        Some(receiver)
    } else {
        None
    };

    if let Some(cron) = cron.as_ref() {
        tasks.spawn(send_request(
            cron.request(&mut SystemTimestamp, CronKind::Start),
        ));
    }

    let heartbeat = cli.heartbeat().map(|config| {
        let token = CancellationToken::new();
        tasks.spawn(heartbeat_loop(config, token.clone()));
        token
    });

    tasks.spawn(log_loop(log, stdout, stderr));

    let exit_status = forward_signals_and_wait(child).await?;

    debug!("command exited with: {}", exit_status);

    if exit_status.success() {
        if let Some(cron) = cron.as_ref() {
            tasks.spawn(send_request(
                cron.request(&mut SystemTimestamp, CronKind::Finish),
            ));
        }
    }

    if let Some(heartbeat) = heartbeat {
        heartbeat.cancel();
    }

    tasks.close();

    if !tasks.is_empty() {
        debug!("waiting for {} tasks to complete", tasks.len());

        // Calling `forward_signals_and_wait` earlier set a signal handler for those signals,
        // overriding their default behaviour, which is to cause the process to terminate.
        // After `forward_signals_and_wait` finishes, those signal handlers are still set.
        //
        // While we wait for the tasks to complete, we need to continue to listen to those
        // signal handlers.
        //
        // This allows for the wrapper process to be terminated by certain signals both before
        // and after the child process' lifetime.
        //
        // See https://docs.rs/tokio/latest/tokio/signal/unix/struct.Signal.html#caveats
        // for reference.
        let mut signals = signal_stream()?;

        loop {
            select! {
                biased;

                _ = tasks.wait() => {
                    break;
                }

                Some(signal) = signals.next() => {
                    if has_terminating_intent(&signal) {
                        debug!("received terminating signal after child: {}", signal);
                        return Ok(128 + signal as i32);
                    } else {
                        trace!("ignoring non-terminating signal after child: {}", signal);
                    }
                }
            }
        }
    }

    if let Some(code) = exit_status.code() {
        Ok(code)
    } else {
        match exit_status.signal() {
            Some(signal) => Ok(128 + signal),
            None => Err("command exited without code or signal".into()),
        }
    }
}
