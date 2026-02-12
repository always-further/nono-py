"""Pytest configuration and fixtures."""

import pytest  # ty:ignore[unresolved-import]  # noqa: F401


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
