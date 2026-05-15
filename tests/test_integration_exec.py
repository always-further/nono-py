"""Integration tests for end-to-end sandboxed execution flows."""

import json
from pathlib import Path

import pytest
from conftest import add_system_paths

from nono_py import (
    AccessMode,
    CapabilitySet,
    QueryContext,
    SandboxState,
    load_policy,
    sandboxed_exec,
)


@pytest.mark.integration
@pytest.mark.usefixtures("require_sandboxed_exec")
class TestPolicyToExec:
    def test_policy_resolve_then_exec(self, temp_dir: Path) -> None:
        """Policy load → resolve_groups → sandboxed_exec succeeds end-to-end."""
        policy_doc = {
            "groups": {
                "test_read": {
                    "description": "Temp dir read",
                    "allow": {"read": [str(temp_dir)]},
                }
            }
        }
        policy = load_policy(json.dumps(policy_doc))
        caps = CapabilitySet()
        add_system_paths(caps)
        policy.resolve_groups(["test_read"], caps)

        result = sandboxed_exec(caps, ["echo", "policy-ok"], cwd=str(temp_dir))
        assert result.exit_code == 0
        assert b"policy-ok" in result.stdout

    def test_policy_write_group_allows_file_creation(self, temp_dir: Path) -> None:
        """A write-access group allows the child to create files."""
        policy_doc = {
            "groups": {
                "test_write": {
                    "description": "Temp dir write",
                    "allow": {"read_write": [str(temp_dir)]},
                }
            }
        }
        policy = load_policy(json.dumps(policy_doc))
        caps = CapabilitySet()
        add_system_paths(caps)
        policy.resolve_groups(["test_write"], caps)

        out_file = temp_dir / "policy_output.txt"
        result = sandboxed_exec(
            caps,
            ["bash", "-c", f"echo written > {out_file}"],
            cwd=str(temp_dir),
        )
        assert result.exit_code == 0
        assert out_file.read_text().strip() == "written"


@pytest.mark.integration
class TestSandboxStateTransfer:
    @pytest.mark.usefixtures("require_sandboxed_exec")
    def test_round_trip_then_exec(self, temp_dir: Path) -> None:
        """CapabilitySet → SandboxState JSON → restored CapabilitySet still runs commands."""
        caps = CapabilitySet()
        add_system_paths(caps)
        caps.allow_path(str(temp_dir), AccessMode.READ_WRITE)

        json_str = SandboxState.from_caps(caps).to_json()
        restored_caps = SandboxState.from_json(json_str).to_caps()

        result = sandboxed_exec(restored_caps, ["echo", "transferred"], cwd=str(temp_dir))
        assert result.exit_code == 0
        assert b"transferred" in result.stdout

    def test_net_blocked_preserved_through_round_trip(self, temp_dir: Path) -> None:
        """block_network() survives a SandboxState JSON round-trip."""
        caps = CapabilitySet()
        caps.allow_path(str(temp_dir), AccessMode.READ)
        caps.block_network()

        json_str = SandboxState.from_caps(caps).to_json()
        restored = SandboxState.from_json(json_str)
        assert restored.net_blocked is True
        restored_caps = restored.to_caps()
        assert restored_caps.is_network_blocked is True


@pytest.mark.integration
class TestQueryThenExecConsistency:
    @pytest.mark.usefixtures("require_sandboxed_exec")
    def test_allowed_path_is_readable_in_sandbox(
        self, temp_dir: Path, temp_file: Path
    ) -> None:
        """QueryContext says allowed → sandboxed_exec can read the same path."""
        caps = CapabilitySet()
        add_system_paths(caps)
        caps.allow_path(str(temp_dir), AccessMode.READ_WRITE)

        qc = QueryContext(caps)
        query_result = qc.query_path(str(temp_file), AccessMode.READ)
        assert query_result["status"] == "allowed"

        result = sandboxed_exec(caps, ["cat", str(temp_file)], cwd=str(temp_dir))
        assert result.exit_code == 0
        assert b"test content" in result.stdout

    def test_network_denied_reflected_in_query(self, temp_dir: Path) -> None:
        """After block_network(), QueryContext.query_network() reports denied."""
        caps = CapabilitySet()
        caps.allow_path(str(temp_dir), AccessMode.READ)
        caps.block_network()

        qc = QueryContext(caps)
        result = qc.query_network()
        assert result["status"] == "denied"
