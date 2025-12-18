use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

fn pipe_to_command(program: &str, args: &[&str], contents: &str) -> Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("Failed to spawn clipboard helper '{program}'"))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to open stdin for clipboard helper '{program}'"))?;
        stdin
            .write_all(contents.as_bytes())
            .with_context(|| format!("Failed to write to clipboard helper '{program}'"))?;
    }

    let status = child
        .wait()
        .with_context(|| format!("Failed to wait for clipboard helper '{program}'"))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Clipboard helper '{program}' exited with {status}"))
    }
}

fn try_pipe_to_command(program: &str, args: &[&str], contents: &str) -> Result<bool> {
    match pipe_to_command(program, args, contents) {
        Ok(()) => Ok(true),
        Err(e) => {
            if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                if io_err.kind() == std::io::ErrorKind::NotFound {
                    return Ok(false);
                }
            }
            Err(e)
        }
    }
}

pub fn set_clipboard(contents: &str) -> Result<()> {
    if contents.is_empty() {
        return Err(anyhow!("Nothing to copy to clipboard (empty output)"));
    }

    #[cfg(target_os = "macos")]
    {
        if try_pipe_to_command("pbcopy", &[], contents)? {
            return Ok(());
        }
        return Err(anyhow!("Could not find 'pbcopy' to set clipboard"));
    }

    #[cfg(target_os = "windows")]
    {
        // `clip` is typically available via `cmd.exe`.
        // Running `cmd /C clip` is more reliable than invoking `clip` directly.
        pipe_to_command("cmd", &["/C", "clip"], contents)?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Wayland
        if try_pipe_to_command("wl-copy", &[], contents)? {
            return Ok(());
        }
        // X11
        if try_pipe_to_command("xclip", &["-selection", "clipboard"], contents)? {
            return Ok(());
        }
        if try_pipe_to_command("xsel", &["--clipboard", "--input"], contents)? {
            return Ok(());
        }

        return Err(anyhow!(
            "No clipboard helper found. Install one of: wl-copy, xclip, xsel."
        ));
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", unix)))]
    {
        let _ = contents;
        Err(anyhow!("Clipboard is not supported on this platform"))
    }
}

