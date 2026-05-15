"""Pytest configuration and fixtures."""

import contextlib
import pathlib
import sys

import pytest  # ty:ignore[unresolved-import]  # noqa: F401

from nono_py import AccessMode, CapabilitySet, sandboxed_exec

_SYSTEM_PATHS = ["/usr", "/bin", "/sbin", "/lib"]
_MACOS_PATHS = ["/private", "/Library/Frameworks", "/dev"]


def add_system_paths(caps: CapabilitySet) -> None:
    """Add system paths to a capability set, ignoring missing ones."""
    for sys_path in _SYSTEM_PATHS + _MACOS_PATHS:
        with contextlib.suppress(FileNotFoundError):
            caps.allow_path(sys_path, AccessMode.READ)
    # Allow the Python installation so tests can exec sys.executable in the sandbox.
    # Resolves symlinks so venv -> toolcache paths are covered on CI.
    py_prefix = str(pathlib.Path(sys.executable).resolve().parent.parent)
    with contextlib.suppress(FileNotFoundError):
        caps.allow_path(py_prefix, AccessMode.READ)
    # Also allow the venv prefix (pyvenv.cfg, venv site-packages) when running in a venv.
    if sys.prefix != py_prefix:
        with contextlib.suppress(FileNotFoundError):
            caps.allow_path(sys.prefix, AccessMode.READ)


@pytest.fixture
def temp_dir(tmp_path):
    """Provide a temporary directory for tests."""
    return tmp_path


@pytest.fixture
def temp_file(tmp_path):
    """Provide a temporary file for tests."""
    file_path = tmp_path / "test_file.txt"
    file_path.write_text("test content")
    return file_path


@pytest.fixture(scope="session")
def _sandboxed_exec_available(tmp_path_factory: pytest.TempPathFactory) -> bool:
    """Return True if sandboxed_exec can initialize in this process.

    Seatbelt on macOS prohibits nested sandboxing, so this returns False when
    tests run inside an existing nono (or other Seatbelt) sandbox.
    """
    tmp = tmp_path_factory.mktemp("exec_probe")
    caps = CapabilitySet()
    add_system_paths(caps)
    caps.allow_path(str(tmp), AccessMode.READ_WRITE)
    result = sandboxed_exec(caps, ["true"], cwd=str(tmp))
    return result.exit_code != 126


@pytest.fixture
def require_sandboxed_exec(_sandboxed_exec_available: bool) -> None:
    """Skip the test if sandboxed_exec cannot initialize (nested sandbox)."""
    if not _sandboxed_exec_available:
        pytest.skip("sandboxed_exec unavailable in nested sandbox environment")


@pytest.fixture
def session_dir(tmp_path):
    """Empty session directory for audit log tests."""
    d = tmp_path / "session"
    d.mkdir()
    return d


@pytest.fixture
def snapshot_session_dir(tmp_path):
    """Empty session directory for SnapshotManager tests."""
    d = tmp_path / "snap_session"
    d.mkdir()
    return d


@pytest.fixture
def tracked_dir(tmp_path):
    """Tracked directory pre-populated with two seed files."""
    d = tmp_path / "tracked"
    d.mkdir()
    (d / "file_a.txt").write_text("content_a")
    (d / "file_b.txt").write_text("content_b")
    return d
