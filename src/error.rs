use std::fmt;

/// Custom error types for Nothing
#[derive(Debug)]
pub enum NothingError {
    /// Insufficient privileges to access raw volume
    InsufficientPrivileges,

    /// Drive not found
    DriveNotFound(char),

    /// Volume access error
    VolumeAccessError(String),

    /// MFT parsing error
    MftParseError(String),

    /// Generic I/O error
    IoError(std::io::Error),
}

impl fmt::Display for NothingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NothingError::InsufficientPrivileges => {
                write!(
                    f,
                    "Insufficient privileges. Please run as Administrator.\n\
                     Right-click PowerShell and select 'Run as Administrator'"
                )
            }
            NothingError::DriveNotFound(drive) => {
                write!(f, "Drive {}:\\ not found", drive)
            }
            NothingError::VolumeAccessError(msg) => {
                write!(f, "Volume access error: {}", msg)
            }
            NothingError::MftParseError(msg) => {
                write!(f, "MFT parsing error: {}", msg)
            }
            NothingError::IoError(err) => {
                write!(f, "I/O error: {}", err)
            }
        }
    }
}

impl std::error::Error for NothingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NothingError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for NothingError {
    fn from(err: std::io::Error) -> Self {
        NothingError::IoError(err)
    }
}
