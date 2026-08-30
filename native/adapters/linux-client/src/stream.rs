//! Direct connection to the one Linux endpoint carried by a validated ANLI record.

use std::{
    io::{self, Read, Write},
    os::{
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixStream},
    },
};

use crate::LinuxBootstrapInvitation;

/// One connected stream opened only from a validated host invitation.
pub struct LinuxClientStream(UnixStream);

impl LinuxClientStream {
    /// Connects only to the abstract Unix endpoint sealed inside an invitation.
    pub fn connect(invitation: &LinuxBootstrapInvitation) -> io::Result<Self> {
        let address = SocketAddr::from_abstract_name(invitation.endpoint_name().as_bytes())?;
        UnixStream::connect_addr(&address).map(Self)
    }
}

impl Read for LinuxClientStream {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.0.read(output)
    }
}

impl Write for LinuxClientStream {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        self.0.write(input)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl std::fmt::Debug for LinuxClientStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LinuxClientStream(..)")
    }
}
