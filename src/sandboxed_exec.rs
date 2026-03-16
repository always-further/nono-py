//! Sandboxed command execution via fork + sandbox + exec.
//!
//! Provides `sandboxed_exec()` which forks the current process, applies
//! OS-level sandbox restrictions in the child, then exec's a command.
//! The parent captures stdout/stderr and waits for exit. The calling
//! process remains unsandboxed and can call this repeatedly.

use crate::CapabilitySet;
use nono::Sandbox;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::ffi::CString;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Result of a sandboxed command execution.
///
/// Attributes:
///     stdout: Raw bytes from the child's stdout
///     stderr: Raw bytes from the child's stderr
///     exit_code: Process exit code (0 = success, -N = killed by signal N)
#[pyclass(frozen)]
pub struct ExecResult {
    #[pyo3(get)]
    pub stdout: Vec<u8>,
    #[pyo3(get)]
    pub stderr: Vec<u8>,
    #[pyo3(get)]
    pub exit_code: i32,
}

#[pymethods]
impl ExecResult {
    fn __repr__(&self) -> String {
        format!(
            "ExecResult(exit_code={}, stdout_len={}, stderr_len={})",
            self.exit_code,
            self.stdout.len(),
            self.stderr.len()
        )
    }
}

/// Execute a command in a sandboxed child process.
///
/// Forks the current process, applies capability-based sandbox restrictions
/// (Landlock on Linux, Seatbelt on macOS) in the child, then exec's the
/// command. The parent captures stdout/stderr via pipes and waits for exit.
///
/// The calling process remains unsandboxed and can call this repeatedly
/// with different capabilities.
///
/// Args:
///     caps: Capability set defining the child's permitted operations
///     command: List of command + arguments (e.g., ["bash", "-c", "ls /"])
///     cwd: Working directory for the child (defaults to current directory)
///     timeout_secs: Maximum execution time in seconds (None = no limit)
///     env: Optional list of (key, value) tuples for environment variables.
///         When provided, the child inherits the current environment with
///         these variables added or overridden.
///
/// Returns:
///     ExecResult with stdout, stderr, and exit_code
///
/// Raises:
///     RuntimeError: If fork fails, sandbox cannot be applied, or the
///         command cannot be executed
///     ValueError: If the command list is empty
#[pyfunction]
#[pyo3(signature = (caps, command, cwd=None, timeout_secs=None, env=None))]
pub fn sandboxed_exec(
    py: Python<'_>,
    caps: &CapabilitySet,
    command: Vec<String>,
    cwd: Option<String>,
    timeout_secs: Option<f64>,
    env: Option<Vec<(String, String)>>,
) -> PyResult<ExecResult> {
    if command.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "command must not be empty",
        ));
    }

    // Verify single-threaded before fork. Python's GIL means we're called
    // from a Python thread, but there may be other OS threads (GC, etc.).
    // We check /proc/self/status on Linux; on macOS we skip this check
    // since the Seatbelt sandbox_init() path is safe in the child.
    #[cfg(target_os = "linux")]
    {
        let thread_count = get_thread_count().map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to check thread count: {}", e))
        })?;
        // Python typically has multiple threads (GC, etc.) so we allow
        // a reasonable count. The child calls Sandbox::apply() which
        // allocates, but only the forking thread continues in the child.
        // This is safe as long as no other thread holds the allocator lock.
        if thread_count > 32 {
            return Err(PyRuntimeError::new_err(format!(
                "Too many threads ({}) for safe fork. Reduce thread count before calling sandboxed_exec.",
                thread_count
            )));
        }
    }

    // Resolve program to absolute path via PATH lookup before fork.
    // execve does not search PATH, so we must do it ourselves.
    let resolved_program = resolve_program(&command[0])?;
    let program_c = CString::new(resolved_program.as_os_str().as_bytes())
        .map_err(|_| PyRuntimeError::new_err("Program path contains null byte"))?;
    let mut argv_c: Vec<CString> = Vec::with_capacity(command.len());
    for arg in &command {
        argv_c.push(
            CString::new(arg.as_bytes())
                .map_err(|_| PyRuntimeError::new_err("Argument contains null byte"))?,
        );
    }

    // Build environment
    let env_c = build_env_cstrings(env.as_deref())?;

    // Convert cwd — canonicalize to handle macOS symlinks (/var -> /private/var)
    let cwd_c = match &cwd {
        Some(d) => {
            let canonical = std::fs::canonicalize(d).map_err(|e| {
                PyRuntimeError::new_err(format!("Cannot resolve working directory '{}': {}", d, e))
            })?;
            Some(
                CString::new(canonical.as_os_str().as_bytes())
                    .map_err(|_| PyRuntimeError::new_err("Working directory contains null byte"))?,
            )
        }
        None => None,
    };

    // Create pipes for stdout and stderr
    let (stdout_read_fd, stdout_write_fd) = create_pipe()?;
    let (stderr_read_fd, stderr_write_fd) = create_pipe()?;

    // Release the GIL during fork+wait so other Python threads can proceed
    let result = py.allow_threads(|| {
        do_fork_sandbox_exec(
            &caps.inner,
            &program_c,
            &argv_c,
            &env_c,
            cwd_c.as_ref(),
            stdout_read_fd,
            stdout_write_fd,
            stderr_read_fd,
            stderr_write_fd,
            timeout_secs,
        )
    });

    result
}

/// Build environment CStrings from current env + overrides.
fn build_env_cstrings(overrides: Option<&[(String, String)]>) -> PyResult<Vec<CString>> {
    let mut env_c: Vec<CString> = Vec::new();

    // Collect override keys for filtering
    let override_keys: std::collections::HashSet<&str> = overrides
        .map(|ovr| ovr.iter().map(|(k, _)| k.as_str()).collect())
        .unwrap_or_default();

    // Copy current environment, skipping overridden keys
    for (key, value) in std::env::vars() {
        if !override_keys.contains(key.as_str()) {
            if let Ok(cstr) = CString::new(format!("{}={}", key, value)) {
                env_c.push(cstr);
            }
        }
    }

    // Add overrides
    if let Some(ovr) = overrides {
        for (key, value) in ovr {
            if let Ok(cstr) = CString::new(format!("{}={}", key, value)) {
                env_c.push(cstr);
            }
        }
    }

    Ok(env_c)
}

/// Create a pipe, returning (read_fd, write_fd).
fn create_pipe() -> PyResult<(i32, i32)> {
    let mut fds = [0i32; 2];
    // SAFETY: pipe() is safe with a valid 2-element array.
    let ret = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if ret != 0 {
        return Err(PyRuntimeError::new_err(format!(
            "pipe() failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok((fds[0], fds[1]))
}

/// Fork, apply sandbox in child, exec command, capture output in parent.
fn do_fork_sandbox_exec(
    caps: &nono::CapabilitySet,
    program_c: &CString,
    argv_c: &[CString],
    env_c: &[CString],
    cwd_c: Option<&CString>,
    stdout_read_fd: i32,
    stdout_write_fd: i32,
    stderr_read_fd: i32,
    stderr_write_fd: i32,
    timeout_secs: Option<f64>,
) -> PyResult<ExecResult> {
    // Build null-terminated pointer arrays for execve
    let argv_ptrs: Vec<*const libc::c_char> = argv_c
        .iter()
        .map(|s| s.as_ptr())
        .chain(std::iter::once(std::ptr::null()))
        .collect();

    let envp_ptrs: Vec<*const libc::c_char> = env_c
        .iter()
        .map(|s| s.as_ptr())
        .chain(std::iter::once(std::ptr::null()))
        .collect();

    // SAFETY: fork() creates a child process. After fork, the child must
    // only call async-signal-safe functions until exec. We allocate
    // (Sandbox::apply generates Seatbelt profiles / opens Landlock path fds)
    // which is technically not async-signal-safe, but is safe in practice
    // because Python's GIL serializes Python threads and we validated the
    // thread context above.
    let pid = unsafe { libc::fork() };

    if pid < 0 {
        // Fork failed - close all pipe fds
        unsafe {
            libc::close(stdout_read_fd);
            libc::close(stdout_write_fd);
            libc::close(stderr_read_fd);
            libc::close(stderr_write_fd);
        }
        return Err(PyRuntimeError::new_err(format!(
            "fork() failed: {}",
            std::io::Error::last_os_error()
        )));
    }

    if pid == 0 {
        // === CHILD PROCESS ===
        child_process(
            caps,
            program_c,
            &argv_ptrs,
            &envp_ptrs,
            cwd_c,
            stdout_read_fd,
            stdout_write_fd,
            stderr_read_fd,
            stderr_write_fd,
        );
        // child_process never returns (calls _exit or execve)
    }

    // === PARENT PROCESS ===
    parent_process(
        pid,
        stdout_read_fd,
        stdout_write_fd,
        stderr_read_fd,
        stderr_write_fd,
        timeout_secs,
    )
}

/// Child process: set up pipes, apply sandbox, chdir, exec.
/// This function never returns.
fn child_process(
    caps: &nono::CapabilitySet,
    program_c: &CString,
    argv_ptrs: &[*const libc::c_char],
    envp_ptrs: &[*const libc::c_char],
    cwd_c: Option<&CString>,
    stdout_read_fd: i32,
    stdout_write_fd: i32,
    stderr_read_fd: i32,
    stderr_write_fd: i32,
) -> ! {
    // Close read ends of pipes (parent reads, child writes)
    unsafe {
        libc::close(stdout_read_fd);
        libc::close(stderr_read_fd);
    }

    // Redirect stdout and stderr to pipe write ends
    unsafe {
        libc::dup2(stdout_write_fd, libc::STDOUT_FILENO);
        libc::dup2(stderr_write_fd, libc::STDERR_FILENO);
        libc::close(stdout_write_fd);
        libc::close(stderr_write_fd);
    }

    // Change working directory if specified
    if let Some(dir) = cwd_c {
        unsafe {
            if libc::chdir(dir.as_ptr()) != 0 {
                let msg = b"nono: failed to chdir\n";
                libc::write(
                    libc::STDERR_FILENO,
                    msg.as_ptr().cast::<libc::c_void>(),
                    msg.len(),
                );
                libc::_exit(126);
            }
        }
    }

    // Apply sandbox restrictions
    if let Err(e) = Sandbox::apply(caps) {
        let detail = format!("nono: sandbox apply failed: {}\n", e);
        let msg = detail.as_bytes();
        unsafe {
            libc::write(
                libc::STDERR_FILENO,
                msg.as_ptr().cast::<libc::c_void>(),
                msg.len(),
            );
            libc::_exit(126);
        }
    }

    // Exec the command
    unsafe {
        libc::execve(program_c.as_ptr(), argv_ptrs.as_ptr(), envp_ptrs.as_ptr());

        // execve only returns on error
        let detail = format!(
            "nono: exec failed: {}\n",
            std::io::Error::last_os_error()
        );
        let msg = detail.as_bytes();
        libc::write(
            libc::STDERR_FILENO,
            msg.as_ptr().cast::<libc::c_void>(),
            msg.len(),
        );
        libc::_exit(127);
    }
}

/// Parent process: close write ends, read output, wait for child.
///
/// Spawns reader threads for stdout/stderr to prevent deadlock when the
/// child produces more output than the pipe buffer. The main thread
/// handles waitpid with timeout.
fn parent_process(
    child_pid: i32,
    stdout_read_fd: i32,
    stdout_write_fd: i32,
    stderr_read_fd: i32,
    stderr_write_fd: i32,
    timeout_secs: Option<f64>,
) -> PyResult<ExecResult> {
    // Close write ends of pipes (child writes, parent reads)
    unsafe {
        libc::close(stdout_write_fd);
        libc::close(stderr_write_fd);
    }

    // Spawn reader threads to drain pipes concurrently.
    // This prevents deadlock when child output exceeds the pipe buffer.
    let stdout_handle = std::thread::spawn(move || {
        // SAFETY: We own this fd and it is a valid pipe read end.
        let mut file = unsafe { std::fs::File::from_raw_fd(stdout_read_fd) };
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf);
        buf
    });

    let stderr_handle = std::thread::spawn(move || {
        // SAFETY: We own this fd and it is a valid pipe read end.
        let mut file = unsafe { std::fs::File::from_raw_fd(stderr_read_fd) };
        let mut buf = Vec::new();
        let _ = file.read_to_end(&mut buf);
        buf
    });

    // Wait for child to exit (with timeout)
    let exit_code = wait_for_child(child_pid, timeout_secs)?;

    // Join reader threads (child is dead, pipes will EOF)
    let stdout_buf = stdout_handle.join().unwrap_or_default();
    let stderr_buf = stderr_handle.join().unwrap_or_default();

    Ok(ExecResult {
        stdout: stdout_buf,
        stderr: stderr_buf,
        exit_code,
    })
}

/// Wait for child process, with optional timeout.
/// Returns the exit code, or -signal_number if killed by signal.
fn wait_for_child(child_pid: i32, timeout_secs: Option<f64>) -> PyResult<i32> {
    let deadline = timeout_secs.map(|t| Instant::now() + Duration::from_secs_f64(t));

    loop {
        let mut status: i32 = 0;
        // SAFETY: waitpid with WNOHANG is safe with a valid pid.
        let ret = unsafe {
            libc::waitpid(
                child_pid,
                &mut status,
                if deadline.is_some() {
                    libc::WNOHANG
                } else {
                    0
                },
            )
        };

        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(PyRuntimeError::new_err(format!(
                "waitpid() failed: {}",
                err
            )));
        }

        if ret == 0 {
            // Child still running (WNOHANG returned 0)
            if let Some(dl) = deadline {
                if Instant::now() >= dl {
                    // Timeout: kill the child
                    unsafe {
                        libc::kill(child_pid, libc::SIGKILL);
                    }
                    // Reap the killed child
                    unsafe {
                        libc::waitpid(child_pid, &mut status, 0);
                    }
                    return Ok(124); // Standard timeout exit code
                }
            }
            // Brief sleep to avoid busy-waiting
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }

        // Child exited
        // SAFETY: These macros may require unsafe on some platforms (Linux)
        // but are safe functions on macOS. Allow both.
        #[allow(unused_unsafe)]
        if unsafe { libc::WIFEXITED(status) } {
            #[allow(unused_unsafe)]
            return Ok(unsafe { libc::WEXITSTATUS(status) });
        }
        #[allow(unused_unsafe)]
        if unsafe { libc::WIFSIGNALED(status) } {
            #[allow(unused_unsafe)]
            return Ok(-(unsafe { libc::WTERMSIG(status) }));
        }

        return Err(PyRuntimeError::new_err(
            "Child process exited with unexpected status",
        ));
    }
}

/// Resolve a program name to its absolute path by searching PATH.
///
/// If the program is already an absolute or relative path, it is returned
/// directly (after checking it exists). Otherwise, searches each directory
/// in the PATH environment variable.
fn resolve_program(program: &str) -> PyResult<PathBuf> {
    let path = Path::new(program);

    // If it contains a path separator, treat as a path (not a bare command)
    if program.contains('/') {
        if path.exists() {
            return Ok(path.to_path_buf());
        }
        return Err(PyRuntimeError::new_err(format!(
            "Program not found: {}",
            program
        )));
    }

    // Search PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = Path::new(dir).join(program);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }

    Err(PyRuntimeError::new_err(format!(
        "Program not found in PATH: {}",
        program
    )))
}

/// Get the number of threads in the current process (Linux only).
#[cfg(target_os = "linux")]
fn get_thread_count() -> Result<usize, String> {
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|e| format!("Cannot read /proc/self/status: {}", e))?;
    for line in status.lines() {
        if let Some(count_str) = line.strip_prefix("Threads:") {
            return count_str
                .trim()
                .parse()
                .map_err(|_| "Cannot parse thread count".to_string());
        }
    }
    Err("Threads field not found in /proc/self/status".to_string())
}
