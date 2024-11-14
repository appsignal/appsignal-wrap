mod check_in;
mod cli;
mod error;
mod log;

mod channel;
mod client;
mod exit;
mod ndjson;
mod package;
mod signal;
mod timestamp;

use crate::channel::{maybe_recv, maybe_spawn_tee};
use crate::check_in::{CronKind, HeartbeatConfig};
use crate::cli::Cli;
use crate::client::send_request;
use crate::log::{LogConfig, LogMessage, LogSeverity};
use crate::package::NAME;
use crate::signal::{has_terminating_intent, signal_stream};
use crate::timestamp::SystemTimestamp;

use ::log::{debug, error, trace};
use error::ErrorConfig;
use std::collections::VecDeque;
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
use tokio::sync::oneshot;
use tokio::time::{interval, Duration, MissedTickBehavior};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use clap::Parser;
use env_logger::Env;

fn main() {
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

#[tokio::main]
async fn start(cli: Cli) -> Result<i32, Box<dyn std::error::Error>> {
    let cron = cli.cron();
    let log = cli.log();
    let error = cli.error();

    let tasks = TaskTracker::new();

    let (child, stdout, stderr) = spawn_child(&cli, &tasks)?;

    let (log_stdout, error_stdout) = maybe_spawn_tee(stdout);
    let (log_stderr, error_stderr) = maybe_spawn_tee(stderr);

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

    tasks.spawn(log_loop(log, log_stdout, log_stderr));

    let error_message = if error.is_some() {
        let (sender, receiver) = oneshot::channel();
        tasks.spawn(error_message_loop(sender, error_stdout, error_stderr));
        Some(receiver)
    } else {
        None
    };

    let exit_status = forward_signals_and_wait(child).await?;

    debug!("command exited with: {}", exit_status);

    if exit_status.success() {
        if let Some(cron) = cron.as_ref() {
            tasks.spawn(send_request(
                cron.request(&mut SystemTimestamp, CronKind::Finish),
            ));
        }
    } else if let Some(error) = error {
        tasks.spawn(send_error_request(
            error,
            exit_status,
            error_message.unwrap(),
        ));
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

type SpawnedChild = (
    Child,
    Option<UnboundedReceiver<String>>,
    Option<UnboundedReceiver<String>>,
);

fn spawn_child(cli: &Cli, tasks: &TaskTracker) -> io::Result<SpawnedChild> {
    let should_stdout = cli.should_pipe_stdout();
    let should_stderr = cli.should_pipe_stderr();

    let mut child = command(&cli.command, should_stdout, should_stderr).spawn()?;

    let stdout = if should_stdout {
        let (sender, receiver) = unbounded_channel();
        tasks.spawn(pipe_lines(child.stdout.take().unwrap(), stdout(), sender));
        Some(receiver)
    } else {
        None
    };

    let stderr = if should_stderr {
        let (sender, receiver) = unbounded_channel();
        tasks.spawn(pipe_lines(child.stderr.take().unwrap(), stderr(), sender));
        Some(receiver)
    } else {
        None
    };

    Ok((child, stdout, stderr))
}

// Pipes lines from an asynchronous reader to a synchronous writer, returning
// a join handle for the task piping the lines, which must not be dropped for
// the lifetime of the process, and a channel receiver that will receive each
// line as it is written.
async fn pipe_lines(
    from: impl AsyncRead + Unpin + Send + 'static,
    mut to: impl Write + Send + 'static,
    sender: UnboundedSender<String>,
) {
    let mut from = BufReader::new(from).lines();

    loop {
        match from.next_line().await {
            Ok(Some(line)) => {
                if let Err(err) = writeln!(to, "{}", line) {
                    debug!("error writing line: {}", err);
                    break;
                }

                if let Err(err) = sender.send(line) {
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

const LOG_MESSAGES_BATCH_SIZE: usize = 100;

async fn log_loop(
    log: LogConfig,
    mut stdout: Option<UnboundedReceiver<String>>,
    mut stderr: Option<UnboundedReceiver<String>>,
) {
    if stdout.is_none() && stderr.is_none() {
        return;
    }

    let mut timestamp = MonotonicTimestamp::new(SystemTimestamp);

    let mut messages = Vec::new();
    let mut interval = interval(Duration::from_secs(10));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let tasks = TaskTracker::new();

    loop {
        if messages.len() >= LOG_MESSAGES_BATCH_SIZE {
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

const ERROR_MESSAGE_LINES: usize = 10;

async fn error_message_loop(
    sender: oneshot::Sender<VecDeque<String>>,
    mut stdout: Option<UnboundedReceiver<String>>,
    mut stderr: Option<UnboundedReceiver<String>>,
) {
    let mut lines = VecDeque::with_capacity(ERROR_MESSAGE_LINES);

    loop {
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
                        if lines.len() >= ERROR_MESSAGE_LINES {
                            lines.pop_front();
                        }

                        lines.push_back(line);
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
                        if lines.len() >= ERROR_MESSAGE_LINES {
                            lines.pop_front();
                        }

                        lines.push_back(line);
                    }
                }
            }

            else => break
        }
    }

    if sender.send(lines).is_err() {
        debug!("error sending error message");
    }
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

async fn send_error_request(
    error: ErrorConfig,
    exit_status: ExitStatus,
    receiver: oneshot::Receiver<VecDeque<String>>,
) {
    let lines = match receiver.await {
        Ok(lines) => lines,
        Err(_) => {
            debug!("error receiving error message");
            VecDeque::new()
        }
    };

    send_request(error.request(&mut SystemTimestamp, &exit_status, lines)).await;
}

fn command(argv: &[String], should_stdout: bool, should_stderr: bool) -> Command {
    let mut command = Command::new(argv[0].clone());
    for arg in argv[1..].iter() {
        command.arg(arg);
    }

    if should_stdout {
        command.stdout(Stdio::piped());
    }

    if should_stderr {
        command.stderr(Stdio::piped());
    }

    unsafe {
        command.pre_exec(exit::exit_with_parent);
    }

    command
}
