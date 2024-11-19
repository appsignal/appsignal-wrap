use std::ffi::OsString;

use crate::check_in::{CheckInConfig, CronConfig, HeartbeatConfig};
use crate::error::ErrorConfig;
use crate::log::{LogConfig, LogOrigin};

use ::log::warn;
use clap::Parser;

/// A wrapper to track the execution of arbitrary processes with AppSignal.
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
#[command(version)]
pub struct Cli {
    /// The AppSignal *app-level* push API key. Required.
    ///
    /// This is the app-level push API key for the AppSignal application
    /// that logs, errors and check-ins will be sent to. This is *not* the
    /// organization-level API key.
    ///
    /// You can find these keys in the AppSignal dashboard:
    /// https://appsignal.com/redirect-to/organization?to=admin/api_keys
    ///
    /// Required unless a log source API key is provided using the
    /// `--log-source` option, and no check-ins or errors are being sent.
    #[arg(
        long,
        env = "APPSIGNAL_APP_PUSH_API_KEY",
        value_name = "APP_PUSH_API_KEY",
        required_unless_present = "log_source"
    )]
    api_key: Option<String>,

    /// The name to use to send check-ins, logs and errors to AppSignal.
    /// Required.
    ///
    /// This value is used as the identifier for cron or heartbeat
    /// check-ins, if either the `--cron` or `--heartbeat` option is set, as
    /// the group for logs, and as the action for errors.
    ///
    /// This name should represent a *kind* of process, not be unique to
    /// the specific invocation of the process. See the `--digest` option for
    /// a unique identifier for this invocation.
    ///
    /// The `--cron`, `--heartbeat`, `--log` and `--error` options can be
    /// used to override this value for each use case.
    #[arg(index = 1, value_name = "NAME", required = true)]
    name: String,

    /// The command to execute. Required.
    ///
    ///
    #[arg(index = 2, allow_hyphen_values = true, last = true, required = true)]
    pub command: Vec<String>,

    /// Send heartbeat check-ins.
    ///
    /// If this option is set, a heartbeat check-in will be sent two times
    /// per minute.
    ///
    /// Optionally, the identifier for the check-in can be provided. If
    /// omitted, the name given as the first argument will be used.
    #[arg(
        long,
        value_name = "IDENTIFIER",
        requires = "api_key",
        conflicts_with = "cron"
    )]
    heartbeat: Option<Option<String>>,

    /// Send cron check-ins.
    ///
    /// If this option is set, a start cron check-in will be sent when the
    /// process starts, and if the wrapped process finishes with a success
    /// exit code, a finish cron check-in will be sent when the process
    /// finishes.
    ///
    /// Optionally, the identifier for the check-in can be provided. If
    /// omitted, the name given as the first argument will be used.
    #[arg(
        long,
        value_name = "IDENTIFIER",
        requires = "api_key",
        conflicts_with = "heartbeat"
    )]
    cron: Option<Option<String>>,

    /// Do not send logs.
    ///
    /// If this option is set, no logs will be sent to AppSignal.
    ///
    /// By default, both standard output and standard error will be sent as
    /// logs. Use the `--no-stdout` and `--no-stderr` options to disable
    /// sending standard output and standard error respectively.
    #[arg(long)]
    no_log: bool,

    /// Do not send errors.
    ///
    /// If this option is set, no errors will be sent to AppSignal.
    ///
    /// By default, an error will be sent to AppSignal if the process fails to
    /// start, or if the process finishes with a non-zero exit code.
    ///
    /// The error message sent to AppSignal will include the last lines of
    /// standard output and standard error, unless the `--no-stdout` or
    /// `--no-stderr` options are set.
    #[arg(long)]
    no_error: bool,

    /// Override the log group to use to send logs.
    ///
    /// If this option is not set, the name given as the first argument will
    /// be used as the log group.
    ///
    /// By default, both standard output and standard error will be sent as
    /// logs. Use the `--no-stdout` and `--no-stderr` options to disable
    /// sending standard output and standard error respectively, or use the
    /// `--no-log` option to disable sending logs entirely.
    #[arg(long, value_name = "GROUP")]
    log: Option<String>,

    /// Override the action name to use to group errors.
    ///
    /// If this option is not set, the name given as the first argument will
    /// be used as the action name.
    ///
    /// By default, an error will be sent to AppSignal if the process fails to
    /// start, or if Use the `--no-error` option to disable sending errors to
    /// AppSignal.
    ///
    /// The error message sent to AppSignal will include the last lines of
    /// standard output and standard error. Use the `--no-stdout` or
    /// `--no-stderr` options are set.
    #[arg(long, value_name = "ACTION", requires = "api_key")]
    error: Option<String>,

    /// The log source API key to use to send logs.
    ///
    /// If this option is not set, logs will be sent to the default
    /// "application" log source for the application specified by the
    /// app-level push API key -- see the `--api-key` option.
    #[arg(
        long,
        env = "APPSIGNAL_LOG_SOURCE_API_KEY",
        value_name = "LOG_SOURCE_API_KEY"
    )]
    log_source: Option<String>,

    /// Do not use standard output in logs or error messages.
    ///
    /// Do not send standard output as logs, and do not use the last
    /// lines of standard output as part of the error message.
    #[arg(long)]
    no_stdout: bool,

    /// Do not use standard error in logs or error messages.
    ///
    /// Do not send standard error as logs, and do not use the last
    /// lines of standard error as part of the error message.
    #[arg(long)]
    no_stderr: bool,

    /// The AppSignal public endpoint to use.
    #[arg(
        long,
        hide = true,
        env = "APPSIGNAL_PUBLIC_ENDPOINT",
        value_name = "PUBLIC_ENDPOINT",
        default_value = "https://appsignal-endpoint.net"
    )]
    endpoint: String,

    /// The hostname to report. Determined automatically.
    ///
    /// This value will be used as the hostname when sending logs, and added
    /// as a tag to errors. We attempt to determine the hostname automatically,
    /// but this configuration option can be used to override it.
    #[arg(
        long,
        env = "APPSIGNAL_HOSTNAME",
        default_value = hostname(),
    )]
    hostname: String,

    /// The digest to uniquely identify this invocation of the process.
    /// Used in cron check-ins as a digest, in logs as an attribute, and in
    /// errors as a tag.
    /// Unless overriden, this value is automatically set to a random value.
    #[arg(
      long,
      hide = true,
      default_value = random_digest(),
      hide_default_value = true
    )]
    digest: String,
}

pub fn hostname() -> String {
    use nix::unistd::gethostname;

    gethostname()
        .ok()
        .and_then(|hostname| OsString::into_string(hostname).ok())
        .unwrap_or_else(|| "unknown".to_string())
}

fn random_digest() -> String {
    use hex::encode;
    use rand::random;

    encode(random::<[u8; 8]>())
}

impl Cli {
    fn log_and_no_log_warning(&self) -> Option<String> {
        let using: Option<&str> = if self.no_log {
            Some("--no-log")
        } else if self.no_stdout && self.no_stderr {
            Some("--no-stdout and --no-stderr")
        } else {
            None
        };

        let alongside = if self.log.is_some() {
            Some("--log")
        } else if self.log_source.is_some() {
            Some("--log-source")
        } else {
            None
        };

        match (using, alongside) {
            (Some(using), Some(alongside)) => Some(format!(
                "using {using} alongside {alongside}; \
                no logs will be sent to AppSignal"
            )),
            _ => None,
        }
    }

    fn error_and_no_error_warning(&self) -> Option<String> {
        if self.no_error && self.error.is_some() {
            return Some(
                "using --no-error alongside --error; \
                no errors will be sent to AppSignal"
                    .to_string(),
            );
        };

        None
    }

    fn no_log_and_no_data_warning(&self) -> Option<String> {
        if !self.no_error {
            return None;
        }

        let no_checkins: bool = self.cron.is_none() && self.heartbeat.is_none();

        if no_checkins {
            let using: Option<&str> = if self.no_log {
                Some("--no-log and --no-error")
            } else if self.no_stdout && self.no_stderr {
                Some("--no-stdout, --no-stderr and --no-error")
            } else {
                None
            };

            if let Some(using) = using {
                return Some(format!(
                    "using {using} without either --cron or --heartbeat; \
                    no data will be sent to AppSignal"
                ));
            }
        }

        None
    }

    fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if let Some(warning) = self.log_and_no_log_warning() {
            warnings.push(warning);
        }

        if let Some(warning) = self.error_and_no_error_warning() {
            warnings.push(warning);
        }

        if let Some(warning) = self.no_log_and_no_data_warning() {
            warnings.push(warning);
        }

        warnings
    }

    pub fn warn(&self) {
        for warning in self.warnings() {
            warn!("{}", warning);
        }
    }

    pub fn cron(&self) -> Option<CronConfig> {
        match (self.api_key.as_ref(), self.cron.as_ref()) {
            (Some(api_key), Some(identifier)) => Some(CronConfig {
                check_in: CheckInConfig {
                    api_key: api_key.clone(),
                    endpoint: self.endpoint.clone(),
                    identifier: identifier.as_ref().unwrap_or(&self.name).clone(),
                },
                digest: self.digest.clone(),
            }),
            _ => None,
        }
    }

    pub fn heartbeat(&self) -> Option<HeartbeatConfig> {
        match (self.api_key.as_ref(), self.heartbeat.as_ref()) {
            (Some(api_key), Some(identifier)) => Some(HeartbeatConfig {
                check_in: CheckInConfig {
                    api_key: api_key.clone(),
                    endpoint: self.endpoint.clone(),
                    identifier: identifier.as_ref().unwrap_or(&self.name).clone(),
                },
            }),
            _ => None,
        }
    }

    pub fn log(&self) -> LogConfig {
        let api_key = self
            .log_source
            .as_ref()
            .or(self.api_key.as_ref())
            .unwrap()
            .clone();
        let endpoint = self.endpoint.clone();
        let origin = self.log_origin();
        let group = self.log.as_ref().unwrap_or(&self.name).clone();
        let hostname = self.hostname.clone();
        let digest = self.digest.clone();
        let command = self.command_as_str();

        LogConfig {
            api_key,
            endpoint,
            origin,
            hostname,
            group,
            digest,
            command,
        }
    }

    pub fn error(&self) -> Option<ErrorConfig> {
        if self.no_error {
            return None;
        }

        let api_key = self.api_key.as_ref().unwrap().clone();
        let endpoint = self.endpoint.clone();
        let action = self.error.as_ref().unwrap_or(&self.name).clone();
        let hostname = self.hostname.clone();
        let digest = self.digest.clone();
        let command = self.command_as_str();

        Some(ErrorConfig {
            api_key,
            endpoint,
            action,
            hostname,
            digest,
            command,
        })
    }

    fn log_origin(&self) -> LogOrigin {
        LogOrigin::from_args(self.no_log, self.no_stdout, self.no_stderr)
    }

    pub fn should_pipe_stderr(&self) -> bool {
        // If `--error` is set, we need to pipe stderr for the error message,
        // even if we're not sending logs, unless `--no-stderr` is set.
        if self.error.is_some() {
            return !self.no_stderr;
        }

        self.log_origin().is_err()
    }

    pub fn should_pipe_stdout(&self) -> bool {
        // If `--error` is set, we need to pipe stdout for the error message,
        // even if we're not sending logs, unless `--no-stdout` is set.
        if self.error.is_some() {
            return !self.no_stdout;
        }

        self.log_origin().is_out()
    }

    fn command_as_str(&self) -> String {
        self.command.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::NAME;

    // These arguments are required -- without them, the CLI parser will fail.
    fn with_required_args(args: Vec<&str>) -> Vec<&str> {
        let first_args: Vec<&str> = vec![NAME, "some-name", "--api-key", "some-api-key"];
        let last_args: Vec<&str> = vec!["--", "true"];
        first_args
            .into_iter()
            .chain(args)
            .chain(last_args)
            .collect()
    }

    #[test]
    fn random_digest() {
        let digest = super::random_digest();
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(digest.len(), 16);
    }

    #[test]
    fn cli_no_warnings() {
        let cli =
            Cli::try_parse_from(with_required_args(vec![])).expect("failed to parse CLI arguments");

        let warnings = cli.warnings();

        assert!(warnings.is_empty());
    }

    #[test]
    fn cli_warnings_log_and_no_log() {
        for (args, warning) in [(
                vec!["--log", "some-group", "--no-log"],
                "using --no-log alongside --log; no logs will be sent to AppSignal"
            ),
            (
                vec!["--log-source", "some-log-source", "--no-log"],
                "using --no-log alongside --log-source; no logs will be sent to AppSignal"
            ),
            (
                vec!["--log", "some-group", "--no-stdout", "--no-stderr"],
                "using --no-stdout and --no-stderr alongside --log; no logs will be sent to AppSignal"
            ),
            (
                vec!["--log-source", "some-log-source", "--no-stdout", "--no-stderr"],
                "using --no-stdout and --no-stderr alongside --log-source; no logs will be sent to AppSignal"
            )] {
            let cli = Cli::try_parse_from(
                with_required_args(args)
            ).expect("failed to parse CLI arguments");

            let warnings = cli.warnings();

            assert!(!warnings.is_empty());
            assert!(
                warnings.contains(&warning.to_string()),
                "actual: {warnings:?}, expected to contain: {warning:?}"
            );
        }
    }

    #[test]
    fn cli_warnings_error_and_no_error() {
        let args = vec!["--error", "some-action", "--no-error"];
        let cli =
            Cli::try_parse_from(with_required_args(args)).expect("failed to parse CLI arguments");

        let warnings = cli.warnings();

        assert!(warnings.len() == 1);
        assert_eq!(
            warnings[0],
            "using --no-error alongside --error; no errors will be sent to AppSignal"
        );
    }

    #[test]
    fn cli_warnings_no_log_and_no_data() {
        for (args, warning) in [
            (
                vec!["--no-log", "--no-error"],
                Some("using --no-log and --no-error without either --cron or --heartbeat; no data will be sent to AppSignal")
            ),
            (
                vec!["--no-stdout", "--no-stderr", "--no-error"],
                Some("using --no-stdout, --no-stderr and --no-error without either --cron or --heartbeat; no data will be sent to AppSignal")
            ),
            (
                vec!["--no-log", "--no-stdout", "--no-stderr", "--no-error"],
                Some("using --no-log and --no-error without either --cron or --heartbeat; no data will be sent to AppSignal")
            ),
            (
                vec!["--no-log", "--no-error", "--cron"],
                None
            ),
            (
                vec!["--no-log", "--no-error", "--heartbeat"],
                None
            ),
            (
                vec!["--no-log"],
                None
            ),
        ] {
            let cli = Cli::try_parse_from(
                with_required_args(args)

            ).expect("failed to parse CLI arguments");

            let warnings = cli.warnings();

            if let Some(warning) = warning {
                assert_eq!(warnings.len(), 1);
                assert_eq!(warnings[0], warning);
            } else {
                assert!(warnings.is_empty());
            }
        }
    }

    #[test]
    fn cli_log_config() {
        let cli = Cli::try_parse_from(with_required_args(vec![
            "--log",
            "some-group",
            "--hostname",
            "some-hostname",
            "--digest",
            "some-digest",
        ]))
        .expect("failed to parse CLI arguments");

        let log_config = cli.log();

        assert_eq!(log_config.api_key, "some-api-key");
        assert_eq!(log_config.endpoint, "https://appsignal-endpoint.net");
        assert_eq!(log_config.origin, LogOrigin::All);
        assert_eq!(log_config.group, "some-group");
        assert_eq!(log_config.hostname, "some-hostname");
        assert_eq!(log_config.digest, "some-digest");
    }

    #[test]
    fn cli_log_config_no_log_options() {
        for (args, origin) in [
            (vec!["--no-log"], LogOrigin::None),
            (vec!["--no-stdout", "--no-stderr"], LogOrigin::None),
            (vec!["--no-stdout"], LogOrigin::Stderr),
            (vec!["--no-stderr"], LogOrigin::Stdout),
        ] {
            let cli = Cli::try_parse_from(with_required_args(args))
                .expect("failed to parse CLI arguments");

            let log_config = cli.log();

            assert_eq!(log_config.origin, origin);
        }
    }

    #[test]
    fn cli_error_config() {
        for (args, error) in [
            (vec!["--no-error"], None),
            (vec!["--error", "some-action"], Some("some-action")),
            (vec![], Some("some-name")),
        ] {
            let cli = Cli::try_parse_from(with_required_args(
                args.into_iter()
                    .chain(["--hostname", "some-hostname", "--digest", "some-digest"].into_iter())
                    .collect(),
            ))
            .expect("failed to parse CLI arguments");

            let error_config = cli.error();

            if let Some(action) = error {
                let error_config = error_config.expect("expected error config");
                assert_eq!(error_config.action, action);
                assert_eq!(error_config.api_key, "some-api-key");
                assert_eq!(error_config.endpoint, "https://appsignal-endpoint.net");
                assert_eq!(error_config.hostname, "some-hostname");
                assert_eq!(error_config.digest, "some-digest");
            } else {
                assert!(error_config.is_none());
            }
        }
    }

    #[test]
    fn cli_check_in_config() {
        for (args, cron, heartbeat) in [
            (
                vec!["--cron", "some-cron", "--digest", "some-digest"],
                Some("some-cron"),
                None,
            ),
            (
                vec!["--heartbeat", "some-heartbeat"],
                None,
                Some("some-heartbeat"),
            ),
            (
                vec!["--cron", "--digest", "some-digest"],
                Some("some-name"),
                None,
            ),
            (vec!["--heartbeat"], None, Some("some-name")),
            (vec![], None, None),
        ] {
            let cli = Cli::try_parse_from(with_required_args(args))
                .expect("failed to parse CLI arguments");

            let cron_config = cli.cron();
            let heartbeat_config = cli.heartbeat();

            if let Some(identifier) = cron {
                let cron_config = cron_config.expect("expected cron config");
                assert_eq!(cron_config.check_in.identifier, identifier);
                assert_eq!(cron_config.check_in.api_key, "some-api-key");
                assert_eq!(
                    cron_config.check_in.endpoint,
                    "https://appsignal-endpoint.net"
                );
                assert_eq!(cron_config.digest, "some-digest");
            } else {
                assert!(cron_config.is_none());
            }

            if let Some(identifier) = heartbeat {
                let heartbeat_config = heartbeat_config.expect("expected heartbeat config");
                assert_eq!(heartbeat_config.check_in.identifier, identifier);
                assert_eq!(heartbeat_config.check_in.api_key, "some-api-key");
                assert_eq!(
                    heartbeat_config.check_in.endpoint,
                    "https://appsignal-endpoint.net"
                );
            } else {
                assert!(heartbeat_config.is_none());
            }
        }
    }
}
