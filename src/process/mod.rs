#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::CustomProcess;

#[cfg(windows)]
pub use windows::CustomProcess;

use std::fmt;

#[derive(Debug)]
pub enum ProcessSpawnError {
    InvalidProgram {
        program: String,
    },
    InvalidArgument {
        argument: String,
    },
    NotFound {
        program: String,
    },
    PermissionDenied {
        program: String,
    },
    OsError {
        program: String,
        message: String,
    },
}

impl fmt::Display for ProcessSpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProgram { program } => {
                write!(f, "invalid program name: {program:?}")
            }

            Self::InvalidArgument { argument } => {
                write!(f, "invalid argument: {argument:?}")
            }

            Self::NotFound { program } => {
                write!(f, "program not found: {program:?}")
            }

            Self::PermissionDenied { program } => {
                write!(f, "permission denied: {program:?}")
            }

            Self::OsError { program, message } => {
                write!(f, "failed to spawn {program:?}: {message}")
            }
        }
    }
}

impl std::error::Error for ProcessSpawnError {}

pub trait ShellProcess {
    fn spawn(program: &str, args: &[String]) -> Result<Self, ProcessSpawnError>
    where
        Self: Sized;

    fn id(&self) -> u32;

    fn wait(&mut self) -> Result<i32, String>;
}
