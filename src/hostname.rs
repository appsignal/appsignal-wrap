use std::ffi::OsString;
use nix::unistd::gethostname;

pub fn hostname() -> String {
  gethostname().ok()
    .and_then(|hostname| OsString::into_string(hostname).ok())
    .unwrap_or_else(|| "unknown".to_string())
}
