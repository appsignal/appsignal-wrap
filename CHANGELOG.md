# `appsignal-wrap` changelog

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
