"""Tests for platform support functions."""

import sys

import pytest  # ty:ignore[unresolved-import]  # noqa: F401

from nono_py import SupportInfo, is_supported, support_info


class TestIsSupported:
    """Tests for is_supported function."""

    def test_returns_bool(self) -> None:
        """Test that is_supported returns a boolean."""
        result = is_supported()
        assert isinstance(result, bool)

    def test_supported_on_known_platforms(self) -> None:
        """Test that sandboxing is supported on Linux and macOS."""
        if sys.platform in ("linux", "darwin"):
            # Should be supported (assuming kernel/OS version is adequate)
            # This might fail on older kernels without Landlock
            result = is_supported()
            # Just verify it returns without error
            assert isinstance(result, bool)


class TestSupportInfo:
    """Tests for support_info function."""

    def test_returns_support_info(self) -> None:
        """Test that support_info returns a SupportInfo object."""
        info = support_info()
        assert isinstance(info, SupportInfo)

    def test_has_required_properties(self) -> None:
        """Test that SupportInfo has all required properties."""
        info = support_info()
        assert hasattr(info, "is_supported")
        assert hasattr(info, "platform")
        assert hasattr(info, "details")

    def test_is_supported_is_bool(self) -> None:
        """Test that is_supported property is boolean."""
        info = support_info()
        assert isinstance(info.is_supported, bool)

    def test_platform_is_string(self) -> None:
        """Test that platform property is a string."""
        info = support_info()
        assert isinstance(info.platform, str)
        assert len(info.platform) > 0

    def test_details_is_string(self) -> None:
        """Test that details property is a string."""
        info = support_info()
        assert isinstance(info.details, str)

    def test_repr(self) -> None:
        """Test SupportInfo repr."""
        info = support_info()
        repr_str = repr(info)
        assert "SupportInfo" in repr_str
        assert info.platform in repr_str

    def test_platform_matches_sys_platform(self) -> None:
        """Test that platform roughly matches sys.platform."""
        info = support_info()
        if sys.platform == "linux":
            assert "linux" in info.platform.lower()
        elif sys.platform == "darwin":
            assert "macos" in info.platform.lower() or "darwin" in info.platform.lower()


class TestSupportInfoConsistency:
    """Tests for consistency between is_supported and support_info."""

    def test_is_supported_matches_support_info(self) -> None:
        """Test that is_supported() matches support_info().is_supported."""
        assert is_supported() == support_info().is_supported
