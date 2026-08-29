#!/usr/bin/env bash
set -euo pipefail

exec python3 /dev/fd/3 "$@" 3<<'PY'
# SLEY_PYTHON_BEGIN
"""Interactive cancellation and process-reaping proof for sley-mcp-bridge."""

from __future__ import annotations

import json
import os
import select
import subprocess
import sys
import time
from pathlib import Path


root = Path(sys.argv[1]).resolve()
work = Path(sys.argv[2]).resolve()
bridge = root / "bin/sley-mcp-bridge"
fake_tool = root / "scripts/fixtures/sley-mcp-test-tool.sh"


def start(mode: str, marker: Path) -> subprocess.Popen[str]:
    env = os.environ.copy()
    env.update({
        "SLEY_ALLOW_TEST_HOOKS": "1",
        "SLEY_MCP_TEST_SLEY_BIN": str(fake_tool),
        "SLEY_MCP_TEST_FAKE_MODE": mode,
        "SLEY_MCP_TEST_MARKER": str(marker),
        "SLEY_MCP_TOOL_TIMEOUT_SECONDS": "30",
    })
    return subprocess.Popen(
        [str(bridge), "--root", str(root)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        bufsize=1,
        env=env,
        start_new_session=True,
    )


def send(process: subprocess.Popen[str], value: dict) -> None:
    assert process.stdin is not None
    process.stdin.write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
    process.stdin.flush()


def read_response(process: subprocess.Popen[str], timeout: float = 10.0) -> dict:
    assert process.stdout is not None
    ready, _, _ = select.select([process.stdout], [], [], timeout)
    if not ready:
        raise AssertionError("timed out waiting for MCP response")
    line = process.stdout.readline()
    if not line:
        raise AssertionError(f"MCP bridge closed before a response (status={process.poll()})")
    return json.loads(line)


def initialize(process: subprocess.Popen[str]) -> None:
    send(process, {
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "sley-mcp-cancel-test", "version": "1"},
        },
    })
    assert read_response(process)["id"] == 1
    send(process, {"jsonrpc": "2.0", "method": "notifications/initialized"})


def tool_call(request_id: int) -> dict:
    return {
        "jsonrpc": "2.0", "id": request_id, "method": "tools/call",
        "params": {"name": "sley_query", "arguments": {"path": "examples/hello.sley", "kind": "tasks"}},
    }


def cancelled(request_id: int, reason: str) -> dict:
    return {
        "jsonrpc": "2.0", "method": "notifications/cancelled",
        "params": {"requestId": request_id, "reason": reason},
    }


def wait_for(path: Path, timeout: float = 5.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.exists():
            return
        time.sleep(0.01)
    raise AssertionError(f"timed out waiting for {path}")


def finish(process: subprocess.Popen[str]) -> tuple[str, str]:
    assert process.stdin is not None and process.stdout is not None and process.stderr is not None
    process.stdin.close()
    process.stdin = None
    process.wait(timeout=10)
    return process.stdout.read(), process.stderr.read()


marker = work / "active"
process = start("slow", marker)
try:
    initialize(process)
    send(process, tool_call(70))
    wait_for(Path(str(marker) + ".started"))
    child_pid, child_pgid = map(int, Path(str(marker) + ".started").read_text(encoding="utf-8").split())
    assert child_pid > 1 and child_pgid > 1, "fake compiler did not report a bounded process group"

    send(process, tool_call(71))
    send(process, cancelled(71, "queued cancellation"))
    queued = read_response(process)
    assert queued["id"] == 71
    assert queued["result"]["isError"] is True
    assert queued["result"]["content"][0]["text"].startswith("SLEY_MCP_CANCELLED:")

    send(process, cancelled(70, "active cancellation"))
    active = read_response(process)
    assert active["id"] == 70
    assert active["result"]["isError"] is True
    assert active["result"]["content"][0]["text"].startswith("SLEY_MCP_CANCELLED:")
    wait_for(Path(str(marker) + ".terminated"))

    send(process, cancelled(70, "duplicate cancellation"))
    send(process, {"jsonrpc": "2.0", "id": 72, "method": "ping", "params": {}})
    assert read_response(process) == {"jsonrpc": "2.0", "id": 72, "result": {}}
    remaining_stdout, stderr = finish(process)
    assert remaining_stdout == "", "cancelled tool emitted a late duplicate response"
    assert "cancelled queued request 71: queued cancellation" in stderr
    assert "cancelled active request 70: active cancellation" in stderr
    assert "ignored cancellation for completed request 70: duplicate cancellation" in stderr
finally:
    if process.poll() is None:
        process.kill()
        process.wait()

deadline = time.monotonic() + 3
while time.monotonic() < deadline:
    try:
        os.kill(child_pid, 0)
    except ProcessLookupError:
        break
    time.sleep(0.02)
else:
    raise AssertionError("cancelled compiler process was not reaped")
try:
    os.killpg(child_pgid, 0)
except ProcessLookupError:
    pass
else:
    raise AssertionError("cancelled compiler process group remained alive")

fast_marker = work / "fast"
process = start("fast", fast_marker)
try:
    initialize(process)
    send(process, tool_call(80))
    completed = read_response(process)
    assert completed["id"] == 80 and completed["result"]["isError"] is False
    send(process, cancelled(80, "after completion"))
    send(process, {"jsonrpc": "2.0", "id": 81, "method": "ping", "params": {}})
    assert read_response(process) == {"jsonrpc": "2.0", "id": 81, "result": {}}
    remaining_stdout, stderr = finish(process)
    assert remaining_stdout == "", "completed request cancellation emitted a false response"
    assert "ignored cancellation for completed request 80: after completion" in stderr
    assert "cancelled active request 80" not in stderr
finally:
    if process.poll() is None:
        process.kill()
        process.wait()

print("Sley MCP active/queued/completed cancellation lifecycle passed")
# SLEY_PYTHON_END
PY
