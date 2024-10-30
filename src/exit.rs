// A reimplementation of nbdkit's `--exit-with-parent` in unsafe Rust.
// See: https://gitlab.com/nbdkit/nbdkit/-/blob/master/common/utils/exit-with-parent.c

#[cfg(target_os = "linux")]
fn set_exit_with_parent() -> () {
    use libc::{c_int, c_long, SIGTERM};

    extern "C" {
        fn prctl(option: c_int, signal: c_long) -> c_int;
    }

    const PR_SET_PDEATHSIG: c_int = 1;

    unsafe {
        prctl(PR_SET_PDEATHSIG, SIGTERM as c_long);
    }
}

#[cfg(target_os = "macos")]
fn set_exit_with_parent() -> () {
    // macOS does not have the `prctl` function, so we do nothing.
    // This means that the child process will not be terminated when
    // the parent process exits.
}

pub fn exit_with_parent() -> Result<(), std::io::Error> {
    set_exit_with_parent();
    Ok(())
}
