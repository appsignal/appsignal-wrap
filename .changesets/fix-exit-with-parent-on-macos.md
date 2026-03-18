---
bump: patch
type: fix
---

Fix exit with parent on macOS. Previously, when the parent process was terminated by a signal which could not be caught, such as `SIGSTOP` or `SIGKILL`, the child process was not terminated on macOS.
