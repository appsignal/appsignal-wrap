// A reimplementation of nbdkit's `--exit-with-parent` in unsafe Rust.
// See: https://gitlab.com/nbdkit/nbdkit/-/blob/master/common/utils/exit-with-parent.c

#[cfg(target_os = "linux")]
fn set_exit_with_parent() {
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
fn set_exit_with_parent() {
    // macOS lacks `prctl`, so we use `kqueue`/`kevent` with `EVFILT_PROC`/`NOTE_EXIT`
    // to watch for the parent process exiting.
    //
    // Because this runs in `pre_exec` (after fork, before exec), any thread we spawn
    // would be killed by the subsequent `exec` call. Instead, we double-fork a detached
    // watcher process: the intermediate process exits immediately (so the watcher is
    // re-parented to PID 1), and the watcher blocks on `kevent` until either the parent
    // or the child exits.
    use libc::{c_int, close, fork, getpid, getppid, kevent, kqueue, pid_t, waitpid};
    use libc::{EVFILT_PROC, EV_ADD, EV_ENABLE, NOTE_EXIT, SIGTERM};
    use std::mem;

    unsafe {
        let child_pid: pid_t = getpid();
        let parent_pid: pid_t = getppid();

        let intermediate = fork();
        if intermediate < 0 {
            return;
        }

        if intermediate == 0 {
            // Intermediate process: fork the watcher and exit immediately so the
            // watcher is re-parented to init/launchd and won't become a zombie.
            let watcher = fork();
            if watcher < 0 {
                libc::_exit(1);
            }
            if watcher == 0 {
                // Watcher process: use kqueue to watch parent_pid and child_pid.
                let kq = kqueue();
                if kq == -1 {
                    libc::_exit(1);
                }

                let changes: [libc::kevent; 2] = [
                    libc::kevent {
                        ident: parent_pid as libc::uintptr_t,
                        filter: EVFILT_PROC,
                        flags: EV_ADD | EV_ENABLE,
                        fflags: NOTE_EXIT,
                        data: 0,
                        udata: std::ptr::null_mut(),
                    },
                    libc::kevent {
                        ident: child_pid as libc::uintptr_t,
                        filter: EVFILT_PROC,
                        flags: EV_ADD | EV_ENABLE,
                        fflags: NOTE_EXIT,
                        data: 0,
                        udata: std::ptr::null_mut(),
                    },
                ];

                let r = kevent(
                    kq,
                    changes.as_ptr(),
                    2,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                );
                if r == -1 {
                    close(kq);
                    libc::_exit(1);
                }

                loop {
                    let mut res: libc::kevent = mem::zeroed();
                    let n = kevent(kq, std::ptr::null(), 0, &mut res, 1, std::ptr::null());
                    if n <= 0 {
                        break;
                    }
                    if res.ident == parent_pid as libc::uintptr_t {
                        // Parent exited: terminate the child.
                        libc::kill(child_pid, SIGTERM);
                        break;
                    } else if res.ident == child_pid as libc::uintptr_t {
                        // Child exited on its own: nothing to do.
                        break;
                    }
                }

                close(kq);
                libc::_exit(0);
            } else {
                libc::_exit(0);
            }
        } else {
            // Original child: reap the intermediate process, then continue to exec.
            let mut status: c_int = 0;
            waitpid(intermediate, &mut status, 0);
        }
    }
}

pub fn exit_with_parent() -> Result<(), std::io::Error> {
    set_exit_with_parent();
    Ok(())
}
