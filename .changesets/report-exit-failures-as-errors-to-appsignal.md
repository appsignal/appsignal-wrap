---
bump: patch
type: add
---

Report exit failures as errors to AppSignal. Use the `--error` command-line option to report an error to AppSignal when the command exits with a non-zero status code, or when the command fails to start:

```
appsignal-wrap --error backup -- ./backup.sh
```

The name given as the value to the `--error` command-line option will be used to group the errors in AppSignal.
