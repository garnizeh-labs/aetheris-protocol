//! Protocol error types using thiserror.

use crate::types::{ClientId, ComponentKind, NetworkId};

/// Errors from the transport layer.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// The specified client is not connected.
    #[error("Client {0:?} is not connected")]
    ClientNotConnected(ClientId),
    /// The datagram sent is larger than the MTU.
    #[error("Datagram exceeds MTU limit ({size} > {max})")]
    PayloadTooLarge {
        /// The calculated size.
        size: usize,
        /// The MTU limit.
        max: usize,
    },
    /// An underlying I/O error occurred.
    #[error("Transport I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A mutex was poisoned.
    #[error("Lock poisoned")]
    LockPoisoned,
}

/// Errors from the serialization layer.
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    /// The buffer was insufficiently sized.
    #[error("Buffer overflow: need {needed} bytes, have {available}")]
    BufferOverflow {
        /// Needed bytes.
        needed: usize,
        /// Provided bytes.
        available: usize,
    },
    /// The payload could not be parsed cleanly.
    #[error("Malformed payload at byte offset {offset}: {message}")]
    MalformedPayload {
        /// The byte offset at which parsing failed.
        offset: usize,
        /// The descriptive error message.
        message: String,
    },
    /// Discovered a component kind that is not registered.
    #[error("Unknown component kind: {0:?}")]
    UnknownComponent(ComponentKind),
    /// An underlying I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors from the ECS adapter.
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    /// An entity was required but not found.
    #[error("Entity {0:?} not found")]
    EntityNotFound(NetworkId),
    /// The entity already exists.
    #[error("Entity {0:?} already exists")]
    EntityAlreadyExists(NetworkId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error as IoError, ErrorKind};

    #[test]
    fn test_error_displays() {
        // TransportError
        let err1 = TransportError::ClientNotConnected(ClientId(42));
        assert_eq!(err1.to_string(), "Client ClientId(42) is not connected");

        let err2 = TransportError::PayloadTooLarge {
            size: 1500,
            max: 1200,
        };
        assert_eq!(err2.to_string(), "Datagram exceeds MTU limit (1500 > 1200)");

        let err3 = TransportError::Io(IoError::new(ErrorKind::ConnectionReset, "connection reset"));
        assert_eq!(err3.to_string(), "Transport I/O error: connection reset");

        // EncodeError
        let err4 = EncodeError::BufferOverflow {
            needed: 256,
            available: 128,
        };
        assert_eq!(
            err4.to_string(),
            "Buffer overflow: need 256 bytes, have 128"
        );

        let err5 = EncodeError::MalformedPayload {
            offset: 10,
            message: "unexpected EOF".to_string(),
        };
        assert_eq!(
            err5.to_string(),
            "Malformed payload at byte offset 10: unexpected EOF"
        );

        let err6 = EncodeError::UnknownComponent(ComponentKind(99));
        assert_eq!(
            err6.to_string(),
            "Unknown component kind: ComponentKind(99)"
        );

        // WorldError
        let err7 = WorldError::EntityNotFound(NetworkId(123));
        assert_eq!(err7.to_string(), "Entity NetworkId(123) not found");

        let err8 = WorldError::EntityAlreadyExists(NetworkId(456));
        assert_eq!(err8.to_string(), "Entity NetworkId(456) already exists");
    }
}
