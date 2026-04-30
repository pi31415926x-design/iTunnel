//! On Windows, `std::process::Command` spawns a visible console (often PowerShell black box) unless
//! `CREATE_NO_WINDOW` is set. Used for all CLI subprocesses we invoke from the GUI binary.

use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Like [`Command::new`], but hides the child process window on Windows.
pub(crate) fn command_new(program: &str) -> Command {
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}
