//! Polling the file descriptors plugin views register with the host run
//! loop, which many Linux plugin windows use for their event loop.

/// The descriptors in `fds` that are ready now. Never blocks.
pub(crate) fn ready(fds: &[i32]) -> Vec<i32> {
    if fds.is_empty() {
        return Vec::new();
    }
    let mut polls: Vec<libc::pollfd> = fds
        .iter()
        .map(|fd| libc::pollfd {
            fd: *fd,
            events: libc::POLLIN,
            revents: 0,
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
        .map(|p| p.fd)
        .collect()
}
