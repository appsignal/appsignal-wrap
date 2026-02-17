# `appsignal-wrap` changelog

## 0.3.1

_Published on 2026-02-17._

### Added

- Add `--revision` command-line argument. When a revision is given using the `--revision` flag or the `APPSIGNAL_REVISION` environment variable, errors will use that revision, causing them to be grouped in AppSignal with the deployment for that revision.

  The revision value will also be added as an attribute to log lines, allowing them to be filtered by revision.

  (patch [62db865](https://github.com/appsignal/appsignal-wrap/commit/62db86592f119fca40e6a712cbef28e83e797d66))

## 0.3.0

_Published on 2025-03-12._

### Changed

- Rename the application to `appsignal-wrap`. A symlink to the previous `appsignal-run` name is created during installation. (minor [3a31014](https://github.com/appsignal/appsignal-wrap/commit/3a31014b727af426e0eb63b7a5e6eb95dae2c285))

## 0.2.2

_Published on 2024-12-09._

### Changed

- Rename the project to `appsignal-run`. (patch [743f35e](https://github.com/appsignal/appsignal-wrap/commit/743f35e9479c911432adc921eeacfeeddd0815f6))

## 0.2.1

_Published on 2024-11-22._

### Added

- Release macOS builds for Apple Silicon (arm64) and Intel (x86_64).

  Using these builds is discouraged in production environments.

  (patch [856b733](https://github.com/appsignal/appsignal-wrap/commit/856b7339f1b9a5cde85d41ad9bb1ffde99b27357))

## 0.2.0

_Published on 2024-11-19._

### Added

- Add command as error tag and log attribute. When reporting log lines or errors, add the command that was used to spawn the child process (or to attempt to) as a tag or attribute. (patch [90668c3](https://github.com/appsignal/appsignal-wrap/commit/90668c315f3736f75f99028b66e7814429064933))
- Report exit failures as errors to AppSignal. Use the `--error` command-line option to report an error to AppSignal when the command exits with a non-zero status code, or when the command fails to start:

  ```
  appsignal-wrap --error backup -- ./backup.sh
  ```

  The name given as the value to the `--error` command-line option will be used to group the errors in AppSignal.

  (patch [90668c3](https://github.com/appsignal/appsignal-wrap/commit/90668c315f3736f75f99028b66e7814429064933))

### Changed

- Add a required positional argument for the name. This name is used as the identifier for cron and heartbeat check-ins, the group for logs, and the action name for errors.

  This avoids repetition of command-line parameters that represent the name:

  ```sh
  # Before:
  appsignal-wrap \
    --cron backup \
    --error backup \
    --log backup \
    -- ./backup.sh

  # After:
  appsignal-wrap backup \
    --cron \
    -- ./backup.sh
  ```

  It is still possible to override the name for a specific purpose by using the `--log GROUP` and `--error ACTION` arguments, or by passing an identifier to either `--cron` or `--heartbeat`:

  ```sh
  appsignal-wrap mysql \
    --heartbeat db
    -- mysqld
  ```

  Additionally, error sending is now enabled by default (use `--no-error` to disable it) and using both cron and heartbeat check-ins in the same invocation is no longer allowed.

  (minor [90668c3](https://github.com/appsignal/appsignal-wrap/commit/90668c315f3736f75f99028b66e7814429064933))

## 0.1.1

_Published on 2024-11-07._

### Added

- Add `--version` command-line option. (patch [f7d2deb](https://github.com/appsignal/appsignal-wrap/commit/f7d2deb8033c26de93a0ac8f0ae69ac441651f06))
- Add installation script. (patch [f7d2deb](https://github.com/appsignal/appsignal-wrap/commit/f7d2deb8033c26de93a0ac8f0ae69ac441651f06))

### Changed

- Restore the default `SIGPIPE` behaviour as implemented by the Rust standard library, which is to ignore `SIGPIPE` signals. Unless overriden by the child process, this behaviour will be inherited by it. (patch [f7d2deb](https://github.com/appsignal/appsignal-wrap/commit/f7d2deb8033c26de93a0ac8f0ae69ac441651f06))

## 0.1.0

_Published on 2024-11-07._

### Added

- Initial release (minor [6b049a2](https://github.com/appsignal/appsignal-wrap/commit/6b049a2816662a5e5d96a564d209b5bd37f63f26))
