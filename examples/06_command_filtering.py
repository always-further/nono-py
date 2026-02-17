#!/usr/bin/env python3
"""Command allow/block lists.

This example demonstrates command filtering capabilities.
Note: Command filtering is metadata tracked by the CapabilitySet
and can be used by higher-level tools for policy enforcement.
"""

from nono_py import AccessMode, CapabilitySet


def main() -> None:
    caps = CapabilitySet()

    # Allow some safe commands
    safe_commands = ["git", "ls", "cat", "grep", "find", "python"]
    for cmd in safe_commands:
        caps.allow_command(cmd)

    print("Allowed commands:")
    for cmd in safe_commands:
        print(f"  - {cmd}")
    print()

    # Block dangerous commands
    dangerous_commands = ["rm", "dd", "mkfs", "fdisk", "wget", "curl"]
    for cmd in dangerous_commands:
        caps.block_command(cmd)

    print("Blocked commands:")
    for cmd in dangerous_commands:
        print(f"  - {cmd}")
    print()

    # Also set up filesystem restrictions
    caps.allow_path("/tmp", AccessMode.READ_WRITE)
    caps.allow_path("/usr", AccessMode.READ)
    caps.block_network()

    print("Full configuration:")
    print(caps.summary())


def demo_build_environment() -> None:
    """Example: Configure a sandbox for a build environment."""
    print("\n" + "=" * 50)
    print("Build Environment Sandbox")
    print("=" * 50 + "\n")

    caps = CapabilitySet()

    # Build tools
    build_commands = [
        "make",
        "cmake",
        "gcc",
        "g++",
        "clang",
        "rustc",
        "cargo",
        "python",
        "pip",
        "npm",
        "node",
    ]
    for cmd in build_commands:
        caps.allow_command(cmd)

    # Block network tools (builds should be offline)
    network_commands = ["curl", "wget", "ssh", "scp", "rsync"]
    for cmd in network_commands:
        caps.block_command(cmd)

    # Block destructive commands
    destructive_commands = ["rm", "rmdir", "dd", "mkfs"]
    for cmd in destructive_commands:
        caps.block_command(cmd)

    # Filesystem access
    caps.allow_path("/tmp", AccessMode.READ_WRITE)
    caps.allow_path("/usr", AccessMode.READ)
    caps.allow_path("/lib", AccessMode.READ)

    # Block network
    caps.block_network()

    print("Build environment configured:")
    print(f"  Allowed commands: {', '.join(build_commands)}")
    print(f"  Blocked commands: {', '.join(network_commands + destructive_commands)}")
    print(f"  Network: {'blocked' if caps.is_network_blocked else 'allowed'}")


if __name__ == "__main__":
    main()
    demo_build_environment()
