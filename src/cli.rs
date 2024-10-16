use crate::check_in::{CheckInConfig, CronConfig, HeartbeatConfig};
use crate::log::{LogConfig, LogOrigin};
use crate::hostname::hostname;

use clap::Parser;
use ::log::warn;

/// a wrapper to track the execution of arbitrary processes with AppSignal
/// 
/// This wrapper allows an arbitrary process to be executed, sending its
/// standard output and standard error as logs to AppSignal, as well as
/// tracking its lifetime using heartbeat or cron check-ins.
/// 
/// The wrapper is transparent: it passes through standard input to the
/// executed process, it passes through the executed process's standard
/// output and standard error to its own standard output and standard error,
/// and it exits with the executed process's exit code.
#[derive(Debug, Parser)]
pub struct Cli {
    /// The AppSignal *app-level* push API key.
    /// 
    /// Required unless a log source API key is provided (see `--log-source`)
    /// and no check-ins are being sent (see `--cron` and `--heartbeat`)
    #[arg(
        long,
        env = "APPSIGNAL_APP_PUSH_API_KEY",
        value_name = "APP_PUSH_API_KEY",
        required_unless_present = "log_source"
    )]
    api_key: Option<String>,

    /// The log group to use to send logs.
    /// 
    /// If this option is not set, logs will be sent to the "process"
    /// log group.
    /// 
    /// By default, both standard output and standard error will be sent as
    /// logs. Use the --no-stdout and --no-stderr options to disable
    /// sending standard output and standard error respectively, or use the
    /// --no-log option to disable sending logs entirely.
    #[arg(long, value_name = "GROUP")]
    log: Option<String>,

    /// The log source API key to use to send logs.
    /// 
    /// If this option is not set, logs will be sent to the default
    /// "application" log source for the application specified by the
    /// app-level push API key.
    #[arg(
        long,
        env = "APPSIGNAL_LOG_SOURCE_API_KEY",
        value_name = "LOG_SOURCE_API_KEY"
    )]
    log_source: Option<String>,

    /// The identifier to use to send heartbeat check-ins.
    /// 
    /// If this option is set, a heartbeat check-in will be sent two times
    /// per minute.
    #[arg(
      long,
      value_name = "IDENTIFIER",
      requires = "api_key"
    )]
    heartbeat: Option<String>,

    /// The identifier to use to send cron check-ins.
    /// 
    /// If this option is set, a start cron check-in will be sent when the
    /// process starts, and if the wrapped process finishes with a success
    /// exit code, a finish cron check-in will be sent when the process
    /// finishes.
    #[arg(
      long,
      value_name = "IDENTIFIER",
      requires = "api_key"
    )]
    cron: Option<String>,

    /// Do not send standard output as logs.
    #[arg(long)]
    no_stdout: bool,

    /// Do not send standard error as logs.
    #[arg(long)]
    no_stderr: bool,

    /// Do not send any logs.
    #[arg(long)]
    no_log: bool,

    /// The command to execute.
    #[arg(
        allow_hyphen_values = true,
        last = true,
        required = true
    )]
    pub command: Vec<String>,

    #[arg(
      long,
      hide = true,
      env = "APPSIGNAL_PUBLIC_ENDPOINT",
      value_name = "PUBLIC_ENDPOINT",
      default_value = "https://appsignal-endpoint.net"
    )]
    endpoint: String,

    #[arg(
      long,
      env = "APPSIGNAL_HOSTNAME",
    )]
    hostname: Option<String>
}

impl Cli {
    pub fn parse() -> Self {        
        let args: Self = match Parser::try_parse() {
            Ok(args) => args,
            Err(err) => {
                eprintln!("appsignal-wrap: {}", err);
                std::process::exit(2);
            }
        };

        let using = if args.no_log {
          Some("--no-log")
        } else if args.no_stdout && args.no_stderr {
          Some("--no-stdout and --no-stderr")
        } else {
          None
        };

        let alongside = if args.log.is_some() {
          Some("--log")
        } else if args.log_source.is_some() {
          Some("--log-source")
        } else {
          None
        };

        if let (Some(using), Some(alongside)) = (using, alongside) {
          warn!("using {using} alongside {alongside}; \
              no logs will be sent to AppSignal");
        }

        let no_checkins: bool = args.cron.is_none() && args.heartbeat.is_none();

        if no_checkins {
          if let Some(using) = using {
            warn!("using {using} without either --cron or --heartbeat; \
                no data will be sent to AppSignal");
          }
        }

        args
    }

    pub fn cron(&self) -> Option<CronConfig> {
        match (self.api_key.as_ref(), self.cron.as_ref()) {
            (Some(api_key), Some(identifier)) => {
                Some(CronConfig(CheckInConfig {
                    api_key: api_key.clone(),
                    endpoint: self.endpoint.clone(),
                    identifier: identifier.clone(),
                }))
            },
            _ => None,
        }
    }

    pub fn heartbeat(&self) -> Option<HeartbeatConfig> {
        match (self.api_key.as_ref(), self.heartbeat.as_ref()) {
            (Some(api_key), Some(identifier)) => {
                Some(HeartbeatConfig(CheckInConfig {
                    api_key: api_key.clone(),
                    endpoint: self.endpoint.clone(),
                    identifier: identifier.clone(),
                }))
            },
            _ => None,
        }
    }

    pub fn log(&self) -> LogConfig {
        let api_key = self.log_source.as_ref().or(self.api_key.as_ref()).unwrap().clone();
        let endpoint = self.endpoint.clone();
        let origin = LogOrigin::from_args(self.no_log, self.no_stdout, self.no_stderr);
        let group = self.log.clone().unwrap_or_else(|| "process".to_string());
        let hostname = self.hostname.clone().unwrap_or_else(hostname);

        LogConfig {api_key, endpoint, origin, hostname, group}
    }
}
