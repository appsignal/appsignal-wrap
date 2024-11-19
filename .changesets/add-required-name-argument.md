---
bump: minor
type: change
---

Add a required positional argument for the name. This name is used as the identifier for cron and heartbeat check-ins, the group for logs, and the action name for errors.

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
