//! One-client worker lifecycle for the Linux abstract Unix-socket adapter.

use std::{
    io::{self, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    thread,
    time::Duration,
};

use super::{LinuxPipeServer, endpoint};

const READ_BUFFER_BYTES: usize = 4 * 1024;
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CONNECTED_READ_TIMEOUT: Duration = Duration::from_millis(50);

impl LinuxPipeServer {
    /// Serves one peer until its stream reaches end-of-file or one host-owned
    /// failure closes it. Call this only from a dedicated worker thread.
    pub fn serve_one(mut self) -> io::Result<()> {
        if self
            .stop_requested
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Ok(());
        }
        let Some(mut stream) = self.accept_one()? else {
            return Ok(());
        };
        if self
            .stop_requested
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Ok(());
        }
        if !endpoint::is_current_user_peer(&stream)? {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Linux socket peer is not the current user",
            ));
        }
        stream.set_read_timeout(Some(CONNECTED_READ_TIMEOUT))?;
        self.serve_connected_client(&mut stream)
    }

    fn accept_one(&mut self) -> io::Result<Option<UnixStream>> {
        let listener = self.listener.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Linux listener was already consumed",
            )
        })?;
        accept_one(&listener, &self.stop_requested)
    }

    fn serve_connected_client(&mut self, stream: &mut UnixStream) -> io::Result<()> {
        let mut read_buffer = [0_u8; READ_BUFFER_BYTES];
        loop {
            if self
                .stop_requested
                .load(std::sync::atomic::Ordering::Acquire)
            {
                return Ok(());
            }
            let bytes_read = match stream.read(&mut read_buffer) {
                Ok(0) => return Ok(()),
                Ok(bytes_read) => bytes_read,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::Interrupted
                            | io::ErrorKind::TimedOut
                            | io::ErrorKind::WouldBlock
                    ) =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            };
            let responses = self
                .session
                .receive(&read_buffer[..bytes_read])
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Linux socket session ended",
                    )
                })?;
            for response in responses {
                stream.write_all(&response)?;
            }
        }
    }
}

fn accept_one(
    listener: &UnixListener,
    stop_requested: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> io::Result<Option<UnixStream>> {
    loop {
        if stop_requested.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(None);
        }
        match listener.accept() {
            Ok((stream, _)) => return Ok(Some(stream)),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_POLL_INTERVAL);
            }
            Err(error) => return Err(error),
        }
    }
}
