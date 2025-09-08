---
bump: patch
type: add
---

Add `--revision` command-line argument. When a revision is given using the `--revision` flag or the `APPSIGNAL_REVISION` environment variable, errors will use that revision, causing them to be grouped in AppSignal with the deployment for that revision.

The revision value will also be added as an attribute to log lines, allowing them to be filtered by revision.
