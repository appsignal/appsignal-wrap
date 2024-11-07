# `appsignal-wrap`: monitor any process with AppSignal

`appsignal-wrap` is a tool that allows you to monitor any process with AppSignal. You can use it to:

- Send the process' standard output and standard error as logs to AppSignal, to be able to troubleshoot issues.
- Send a start cron check-in to AppSignal when the process starts, and a finish cron check-in if it finishes successfully, to be able to track whether it runs successfully and on schedule.
- Send heartbeat check-ins to AppSignal periodically, for as long as the process is active, to be able to monitor its uptime.

## Usage

To use `appsignal-wrap`, you must provide an app-level API key. You can find the API key for your application in the [push and deploy settings](https://appsignal.com/redirect-to/app?to=api_keys) for your application.

To provide the app-level API key, set it as the value for the `APPSIGNAL_APP_PUSH_API_KEY` environment variable, or pass it as the value for the `--api-key` command-line option.

You must also provide a command to execute. This is the command whose output and lifecycle will be monitored with AppSignal.

## Examples

See `appsignal-wrap --help` for detailed information on all configuration options.

### Monitor your database's uptime with AppSignal

You can use the `--heartbeat` command-line option to send heartbeat check-ins (named `database` in this example) to AppSignal periodically, for as long as the database process is running. This allows you to set up alerts that will notify you if the database is no longer running.

In this example, we'll start `mysqld`, the MySQL server process, using `appsignal-wrap`:

```sh
appsignal-wrap --heartbeat database -- mysqld
```

This invocation can then be added to the `mysql.service` service definition:

```conf
# /usr/lib/systemd/system/mysql.service

[Service]
# Modify the existing ExecStart line to add `appsignal-wrap`
ExecStart=/bin/appsignal-wrap --heartbeat database -- /usr/sbin/mysqld
# Add an environment variable containing the AppSignal app-level push API key
Environment=APPSIGNAL_APP_PUSH_API_KEY=...
```

In addition to the specified heartbeat check-ins, by default `appsignal-wrap` will also send your database process' standard output and standard error as logs to AppSignal. Use the `--no-log` configuration option to disable this behaviour.

### Monitor your cron jobs with AppSignal

You can use the `--cron` command-line option to send cron check-ins (named `backup` in this example) to notify AppSignal when your cron job starts, and when it finishes, if and only if it finishes successfully.

In this example, we'll run `/usr/local/bin/backup.sh`, our custom backup shell script, using `appsignal-wrap`:

```sh
appsignal-wrap --cron backup -- bash /usr/local/bin/backup.sh
```

This invocation can then be added to the `/etc/crontab` file:

```sh
# /etc/crontab

APPSIGNAL_APP_PUSH_API_KEY=...

0 2 * * * /usr/local/bin/backup.sh
```

In addition to the specified cron check-ins, by default `appsignal-wrap` will also send your database process' standard output and standard error as logs to AppSignal. Use the `--no-log` configuration option to disable this behaviour.
