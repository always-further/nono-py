"""Tests for AccessMode enum."""

import pytest

from nono_py import AccessMode


class TestAccessMode:
    """Tests for the AccessMode enum."""

    def test_enum_values_exist(self) -> None:
        """Test that all enum values are accessible."""
        assert AccessMode.READ is not None
        assert AccessMode.WRITE is not None
        assert AccessMode.READ_WRITE is not None

    def test_enum_values_distinct(self) -> None:
        """Test that enum values are distinct."""
        assert AccessMode.READ != AccessMode.WRITE
        assert AccessMode.READ != AccessMode.READ_WRITE
        assert AccessMode.WRITE != AccessMode.READ_WRITE

    def test_repr(self) -> None:
        """Test string representation."""
        assert repr(AccessMode.READ) == "AccessMode.READ"
        assert repr(AccessMode.WRITE) == "AccessMode.WRITE"
        assert repr(AccessMode.READ_WRITE) == "AccessMode.READ_WRITE"

    def test_str(self) -> None:
        """Test human-readable string conversion."""
        assert str(AccessMode.READ) == "read"
        assert str(AccessMode.WRITE) == "write"
        assert str(AccessMode.READ_WRITE) == "read+write"

    def test_hashable(self) -> None:
        """Test that AccessMode values are hashable (can be dict keys)."""
        mode_dict = {
            AccessMode.READ: "r",
            AccessMode.WRITE: "w",
            AccessMode.READ_WRITE: "rw",
        }
        assert mode_dict[AccessMode.READ] == "r"
        assert mode_dict[AccessMode.WRITE] == "w"
        assert mode_dict[AccessMode.READ_WRITE] == "rw"

    def test_equality(self) -> None:
        """Test equality comparison."""
        assert AccessMode.READ == AccessMode.READ
        assert not (AccessMode.READ == AccessMode.WRITE)

    def test_usable_in_set(self) -> None:
        """Test that AccessMode values can be added to sets."""
        modes = {AccessMode.READ, AccessMode.WRITE}
        assert AccessMode.READ in modes
        assert AccessMode.WRITE in modes
        assert AccessMode.READ_WRITE not in modes
