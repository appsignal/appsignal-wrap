---
bump: patch
type: security
---

Reject certificates issued under the eMudhra intermediate certificate authorities whose private keys were [compromised in October 2026](https://bugzilla.mozilla.org/show_bug.cgi?id=2083449), in addition to no longer trusting the root certificates they chain to.