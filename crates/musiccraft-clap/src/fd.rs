//! Polling the file descriptors plugins register (`posix-fd-support`), which
//! many Linux plugin windows use for their event loop.

use clap_sys::ext::posix_fd_support::{
    CLAP_POSIX_FD_ERROR, CLAP_POSIX_FD_READ, CLAP_POSIX_FD_WRITE, clap_posix_fd_flags,
};

/// The descriptors in `fds` that are ready now, with what they are ready
/// for. Never blocks.
pub(crate) fn ready(fds: &[(i32, clap_posix_fd_flags)]) -> Vec<(i32, clap_posix_fd_flags)> {
    if fds.is_empty() {
        return Vec::new();
    }
    let mut polls: Vec<libc::pollfd> = fds
        .iter()
        .map(|(fd, flags)| {
            let mut events = 0;
            if flags & CLAP_POSIX_FD_READ != 0 {
                events |= libc::POLLIN;
            }
            if flags & CLAP_POSIX_FD_WRITE != 0 {
                events |= libc::POLLOUT;
            }
            libc::pollfd {
                fd: *fd,
                events,
                revents: 0,
            }
        })
        .collect();
    // SAFETY: `polls` is a valid array of `polls.len()` entries; a zero
    // timeout makes the call return immediately.
    let n = unsafe { libc::poll(polls.as_mut_ptr(), polls.len() as libc::nfds_t, 0) };
    if n <= 0 {
        return Vec::new();
    }
    polls
        .iter()
        .filter(|p| p.revents != 0)
        .map(|p| {
            let mut flags = 0;
            if p.revents & libc::POLLIN != 0 {
                flags |= CLAP_POSIX_FD_READ;
            }
            if p.revents & libc::POLLOUT != 0 {
                flags |= CLAP_POSIX_FD_WRITE;
            }
            if p.revents & (libc::POLLERR | libc::POLLHUP) != 0 {
                flags |= CLAP_POSIX_FD_ERROR;
            }
            (p.fd, flags)
        })
        .collect()
}
