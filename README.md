# `appsignal-wrap`: monitor any process with AppSignal

`appsignal-wrap` is a tool that allows you to monitor any process with AppSignal. You can use it to:

- Send the process' standard output and standard error as logs to AppSignal, to be able to troubleshoot issues.
- Send a start cron check-in to AppSignal when the process starts, and a finish cron check-in if it finishes successfully, to be able to track whether it runs successfully and on schedule.
- Send heartbeat check-ins to AppSignal periodically, for as long as the process is active, to be able to monitor its uptime.

## Installation

The easiest way to get `appsignal-wrap` in your machine is to run our installation one-liner:

```sh
curl -sSL https://github.com/appsignal/appsignal-wrap/releases/latest/download/install.sh | sh
```

You'll need to run it with super-user privileges -- if you're not running this as root, prefix it with `sudo`.

`appsignal-wrap` is only supported for Linux and macOS, in the x86_64 (Intel) and arm64 (Apple Silicon) architectures. Linux distributions based on musl, such as Alpine, are also supported.

Not a fan of `curl | sh` one-liners? Download the binary for your operating system and architecture [from our latest release](https://github.com/appsignal/appsignal-wrap/releases/latest/).

## Usage

See `appsignal-wrap --help` for detailed information on all configuration options.

```
appsignal-wrap NAME [OPTIONS] -- COMMAND
```

To use `appsignal-wrap`, you must provide an app-level API key. You can find the app-level API key in the [push and deploy settings](https://appsignal.com/redirect-to/app?to=api_keys) for your application.

To provide the app-level API key, set it as the value for the `APPSIGNAL_APP_PUSH_API_KEY` environment variable, or pass it as the value for the `--api-key` command-line option.

You must also provide a name as the first argument, which will be used as the identifier for cron and heartbeat check-ins, as the group for logs, and as the action to group errors in AppSignal.

Finally, you must provide a command to execute as the last argument, preceded by `--`. This is the command whose output and lifecycle will be monitored with AppSignal.

### Send standard output and error as logs to AppSignal

By default, `appsignal-wrap` will send the standard output and standard error of the command it executes as logs to AppSignal:

```sh
appsignal-wrap sync_customers -- python ./sync_customers.py
```

The above command will execute `python ./sync_customers.py` with the AppSignal wrapper, sending its standard output and error as logs to AppSignal.

You can disable sending logs entirely by using the `--no-log` command-line option, and you can use `--no-stdout` and `--no-stderr` to control whether standard output and error are used to send logs to AppSignal.

### Report failure exit codes as errors to AppSignal

By default, `appsignal-wrap` will report an error to AppSignal if the command it executes exits with a failure exit code, or if the command fails to be executed:

```sh
appsignal-wrap sync_customers -- python ./sync_customers.py
```

The above command will attempt to execute `python ./sync_customers.py` with the AppSignal wrapper, and it will report an error to AppSignal if it fails to execute the command, or if the command ends with a failure exit code.

You can disable sending errors entirely by using the `--no-error` command-line option.

### Send heartbeat check-ins to AppSignal while your process is running

Use the `--heartbeat` flag to send heartbeat check-ins continuously to AppSignal, for as long as the process is running. This allows you to track that certain processes are always up:

```sh
appsignal-wrap worker --heartbeat -- bundle exec ruby ./worker.rb
```

The above command will execute `bundle exec ruby ./worker.rb`, and send heartbeat check-ins to AppSignal with the `worker` check-in identifier continuously, for as long as the process is running.

It will also send logs and report errors, as described in previous sections. To only send heartbeat check-ins, use `--no-log` and `--no-error`.

### Send cron check-ins to AppSignal when your process starts and finishes

Use the `--cron` flag to send a start cron check-in to AppSignal when the process starts, and a finish cron check-in to AppSignal if it finishes successfully. This allows you to track that certain processes are executed on schedule:

```sh
appsignal-wrap sync_customers --cron -- python ./sync_customers.py
```

The above command will execute `python ./sync_customers.py`, send a start cron check-in to AppSignal with the `sync_customers` check-in identifier if it starts successfully, and send a finish cron check-in to AppSignal if it finishes with a success exit code.

It will also send logs and report errors, as described in previous sections. To only send cron check-ins, use `--no-log` and `--no-error`.

## Examples

### Monitor your database's uptime with AppSignal

You can use the `--heartbeat` command-line option to send heartbeat check-ins (named `database` in this example) to AppSignal periodically, for as long as the database process is running. This allows you to set up alerts that will notify you if the database is no longer running.

In this example, we'll start `mysqld`, the MySQL server process, using `appsignal-wrap`:

```sh
appsignal-wrap database --heartbeat -- mysqld
```

This invocation can then be added to the `mysql.service` service definition:

```sh
# /usr/lib/systemd/system/mysql.service

[Service]
# Modify the existing ExecStart line to add `appsignal-wrap`
ExecStart=/usr/local/bin/appsignal-wrap database --heartbeat -- /usr/sbin/mysqld
# Add an environment variable containing the AppSignal app-level push API key
Environment=APPSIGNAL_APP_PUSH_API_KEY=...
```

In addition to sending heartbeat check-ins, by default `appsignal-wrap` will also: 

- Send your database process' standard output and standard error as logs to AppSignal, under the `database` group
- Report failure exit codes as errors to AppSignal, grouped under the `database` action

You can use the `--no-log` and `--no-error` command-line option to disable this behaviour.

### Monitor your cron jobs with AppSignal

You can use the `--cron` command-line option to send cron check-ins (named `backup` in this example) to notify AppSignal when your cron job starts, and when it finishes, if and only if it finishes successfully.

In this example, we'll run `/usr/local/bin/backup.sh`, our custom backup shell script, using `appsignal-wrap`:

```sh
appsignal-wrap backup --cron -- bash /usr/local/bin/backup.sh
```

This invocation can then be added to the `/etc/crontab` file:

```sh
# /etc/crontab

APPSIGNAL_APP_PUSH_API_KEY=...

0 2 * * * appsignal-wrap backup --cron -- bash /usr/local/bin/backup.sh
```

In addition to sending cron check-ins, by default `appsignal-wrap` will also: 

- Send your database process' standard output and standard error as logs to AppSignal, under the `backup` group
- Report failure exit codes as errors to AppSignal, grouped under the `backup` action

You can use the `--no-log` and `--no-error` command-line option to disable this behaviour.
