use crate::process::{ProcessSpawnError, ShellProcess};

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, GetLastError,
    HANDLE,
};

use windows_sys::Win32::System::Threading::{
    CreateProcessW, GetExitCodeProcess, INFINITE, NORMAL_PRIORITY_CLASS, PROCESS_INFORMATION,
    STARTUPINFOW, WaitForSingleObject,
};

pub struct CustomProcess {
    h_process: HANDLE,
    pid: u32,
}

impl ShellProcess for CustomProcess {
    fn spawn(program: &str, args: &[String]) -> Result<Self, ProcessSpawnError> {
        let mut command_line = String::with_capacity(
            program.len() + args.iter().map(|arg| arg.len() + 1).sum::<usize>(),
        );

        command_line.push_str(program);

        for arg in args {
            command_line.push(' ');
            command_line.push_str(arg);
        }

        let mut command_line: Vec<u16> = OsStr::new(&command_line)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut startup_info: STARTUPINFOW = unsafe { std::mem::zeroed() };
        startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

        let mut process_info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let success = unsafe {
            CreateProcessW(
                std::ptr::null(),
                command_line.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                NORMAL_PRIORITY_CLASS,
                std::ptr::null(),
                std::ptr::null(),
                &startup_info,
                &mut process_info,
            )
        };

        if success != 0 {
            unsafe {
                CloseHandle(process_info.hThread);
            }

            Ok(Self {
                h_process: process_info.hProcess,
                pid: process_info.dwProcessId,
            })
        } else {
            let error_code = unsafe { GetLastError() };

            Err(Self::spawn_error(program, error_code))
        }
    }

    fn id(&self) -> u32 {
        self.pid
    }

    fn wait(&mut self) -> Result<i32, String> {
        unsafe {
            WaitForSingleObject(self.h_process, INFINITE);

            let mut exit_code = 0u32;

            if GetExitCodeProcess(self.h_process, &mut exit_code) != 0 {
                CloseHandle(self.h_process);

                Ok(exit_code as i32)
            } else {
                CloseHandle(self.h_process);

                Err("failed to get process exit code".to_string())
            }
        }
    }
}

impl CustomProcess {
    fn spawn_error(program: &str, error_code: u32) -> ProcessSpawnError {
        match error_code {
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => ProcessSpawnError::NotFound {
                program: program.to_string(),
            },

            ERROR_ACCESS_DENIED => ProcessSpawnError::PermissionDenied {
                program: program.to_string(),
            },

            _ => ProcessSpawnError::OsError {
                program: program.to_string(),
                message: format!("Windows error code: {error_code}"),
            },
        }
    }
}
