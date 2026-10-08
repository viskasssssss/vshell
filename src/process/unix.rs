use crate::process::{ProcessSpawnError, ShellProcess};

use nix::errno::Errno;
use nix::sys::wait::{WaitStatus, waitpid};
use nix::unistd::{ForkResult, Pid, execvp, fork, pipe, read, write};

use std::ffi::CString;
use std::os::fd::AsRawFd;

pub struct CustomProcess {
    pid: Pid,
}

impl ShellProcess for CustomProcess {
    fn spawn(program: &str, args: &[String]) -> Result<Self, ProcessSpawnError> {
        let c_program = CString::new(program).map_err(|_| ProcessSpawnError::InvalidProgram {
            program: program.to_string(),
        })?;

        let mut c_args = Vec::with_capacity(args.len() + 1);
        c_args.push(c_program.clone());

        for arg in args {
            let c_arg =
                CString::new(arg.as_str()).map_err(|_| ProcessSpawnError::InvalidArgument {
                    argument: arg.clone(),
                })?;

            c_args.push(c_arg);
        }

        // The child uses this pipe to report an execvp() error
        // back to the parent.
        let (read_fd, write_fd) = pipe().map_err(|error| ProcessSpawnError::OsError {
            program: program.to_string(),
            message: format!("failed to create error pipe: {error}"),
        })?;

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                // Parent only needs the read end.
                drop(write_fd);

                let mut errno_bytes = [0u8; std::mem::size_of::<i32>()];

                match read(read_fd.as_raw_fd(), &mut errno_bytes) {
                    Ok(0) => {
                        // EOF means execvp() succeeded.
                        drop(read_fd);

                        Ok(Self { pid: child })
                    }

                    Ok(size) if size == errno_bytes.len() => {
                        drop(read_fd);

                        let errno = Errno::from_raw(i32::from_ne_bytes(errno_bytes));

                        Err(Self::spawn_error(program, errno))
                    }

                    Ok(size) => {
                        drop(read_fd);

                        Err(ProcessSpawnError::OsError {
                            program: program.to_string(),
                            message: format!("received incomplete error from child ({size} bytes)"),
                        })
                    }

                    Err(error) => {
                        drop(read_fd);

                        Err(ProcessSpawnError::OsError {
                            program: program.to_string(),
                            message: format!("failed to read child error: {error}"),
                        })
                    }
                }
            }

            Ok(ForkResult::Child) => {
                // Child only needs the write end.
                drop(read_fd);

                match execvp(&c_program, &c_args) {
                    Ok(_) => unreachable!("execvp returned successfully"),

                    Err(error) => {
                        let errno = (error as i32).to_ne_bytes();

                        // nix 0.29's write() accepts anything implementing
                        // AsFd, so we pass OwnedFd directly.
                        let _ = write(&write_fd, &errno);

                        drop(write_fd);

                        std::process::exit(127);
                    }
                }
            }

            Err(error) => {
                drop(read_fd);
                drop(write_fd);

                Err(ProcessSpawnError::OsError {
                    program: program.to_string(),
                    message: format!("fork failed: {error}"),
                })
            }
        }
    }

    fn id(&self) -> u32 {
        self.pid.as_raw() as u32
    }

    fn wait(&mut self) -> Result<i32, String> {
        match waitpid(self.pid, None) {
            Ok(WaitStatus::Exited(_, code)) => Ok(code),

            Ok(WaitStatus::Signaled(_, signal, _)) => {
                Err(format!("process killed by signal: {signal:?}"))
            }

            Ok(status) => Err(format!("process changed state unexpectedly: {status:?}")),

            Err(error) => Err(format!("waitpid failed: {error}")),
        }
    }
}

impl CustomProcess {
    fn spawn_error(program: &str, errno: Errno) -> ProcessSpawnError {
        match errno {
            Errno::ENOENT => ProcessSpawnError::NotFound {
                program: program.to_string(),
            },

            Errno::EACCES => ProcessSpawnError::PermissionDenied {
                program: program.to_string(),
            },

            _ => ProcessSpawnError::OsError {
                program: program.to_string(),
                message: errno.to_string(),
            },
        }
    }
}
