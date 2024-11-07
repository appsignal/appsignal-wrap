---
bump: patch
type: change
---

Restore the default `SIGPIPE` behaviour as implemented by the Rust standard library, which is to ignore `SIGPIPE` signals. Unless overriden by the child process, this behaviour will be inherited by it.
