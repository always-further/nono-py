# nono-py Examples

These examples demonstrate various features of the nono-py sandboxing library.

## Examples

### 01_basic_sandbox.py

Basic sandbox usage: create capabilities, apply sandbox, verify restrictions.

**WARNING**: This example actually applies the sandbox, which is irreversible!

```bash
python examples/01_basic_sandbox.py
```

### 02_query_permissions.py

Test permissions without applying the sandbox using `QueryContext`. Safe to run
repeatedly - no sandbox is applied.

```bash
python examples/02_query_permissions.py
```

### 03_sandbox_state.py

Serialize sandbox configuration to JSON for cross-process transfer or
persistence. Demonstrates the `SandboxState` class.

```bash
python examples/03_sandbox_state.py
```

### 04_capability_inspection.py

Examine capability set contents including filesystem capabilities, access
modes, sources, and deduplication.

```bash
python examples/04_capability_inspection.py
```

### 05_subprocess_sandbox.py

Run untrusted code in a sandboxed subprocess. Shows the pattern for passing
sandbox configuration via environment variables.

**WARNING**: This example applies the sandbox in a subprocess!

```bash
python examples/05_subprocess_sandbox.py
```

### 06_command_filtering.py

Configure command allow/block lists for higher-level policy enforcement.

```bash
python examples/06_command_filtering.py
```

### 07_error_handling.py

Handle errors gracefully: path validation, serialization errors, and
platform support issues.

```bash
python examples/07_error_handling.py
```

## Running Examples

All examples can be run directly:

```bash
# From the repository root
cd examples
python 01_basic_sandbox.py

# Or from anywhere
python /path/to/nono-py/examples/02_query_permissions.py
```

## Platform Support

- **Linux**: Requires kernel 5.13+ with Landlock support
- **macOS**: Uses Seatbelt (App Sandbox)
- **Other**: Not supported

Check support programmatically:

```python
from nono_py import is_supported, support_info

if is_supported():
    info = support_info()
    print(f"Platform: {info.platform}")
else:
    print("Sandboxing not available")
```
