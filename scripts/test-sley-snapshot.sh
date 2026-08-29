#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 /dev/fd/3 "$SCRIPT_DIR" "$@" 3<<'PY'
# SLEY_PYTHON_BEGIN
"""Adversarial tests for the descriptor-relative Sley snapshot sealer."""

from __future__ import annotations

import os
import shutil
import socket
import stat
import tempfile
import types
from pathlib import Path


ROOT = Path(__import__("sys").argv[1]).resolve()
SNAPSHOT_PATH = ROOT / "sley-snapshot.sh"
WRAPPED_SOURCE = SNAPSHOT_PATH.read_text(encoding="utf-8")
PYTHON_SOURCE = WRAPPED_SOURCE.split("# SLEY_PYTHON_BEGIN\n", 1)[1].split("\n# SLEY_PYTHON_END", 1)[0]
SNAPSHOT = types.ModuleType("sley_snapshot")
SNAPSHOT.__file__ = str(SNAPSHOT_PATH)
exec(compile(PYTHON_SOURCE, str(SNAPSHOT_PATH), "exec"), SNAPSHOT.__dict__)


def expect_rejected(kind: str, create) -> None:
    with tempfile.TemporaryDirectory(prefix=f"sley-snapshot-{kind}-") as raw:
        root = Path(raw)
        source = root / "source"
        source.mkdir()
        resource = create(source)
        try:
            SNAPSHOT.seal_snapshot(source, root / "sealed", root / "manifest.json")
        except SNAPSHOT.SnapshotError as exc:
            assert kind.lower() in str(exc).lower(), (kind, str(exc))
        else:
            raise AssertionError(f"{kind} entry was accepted")
        finally:
            if resource is not None:
                resource.close()


expect_rejected("symlink", lambda source: (source / "escape").symlink_to("/etc/passwd"))
expect_rejected("symlink", lambda source: (source / "directory").symlink_to("/tmp", target_is_directory=True))


def create_chain(source: Path):
    (source / "first").symlink_to("second")
    (source / "second").symlink_to("/etc/passwd")


expect_rejected("symlink", create_chain)
expect_rejected("FIFO", lambda source: os.mkfifo(source / "pipe"))


def create_socket(source: Path):
    endpoint = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    endpoint.bind(str(source / "socket"))
    return endpoint


expect_rejected("socket", create_socket)

if hasattr(os, "mknod"):
    try:
        expect_rejected(
            "character device",
            lambda source: os.mknod(source / "device", stat.S_IFCHR | 0o600, os.makedev(1, 3)),
        )
    except PermissionError:
        pass

with tempfile.TemporaryDirectory(prefix="sley-snapshot-race-") as raw:
    root = Path(raw)
    source = root / "source"
    source.mkdir()
    victim = source / "victim.sley"
    victim.write_text("task main() -> Int { return 0 }\n", encoding="utf-8")

    def swap(relative: str) -> None:
        if relative == "victim.sley" and victim.exists() and not victim.is_symlink():
            victim.unlink()
            victim.symlink_to("/etc/passwd")

    SNAPSHOT._before_open = swap
    try:
        SNAPSHOT.seal_snapshot(source, root / "sealed", root / "manifest.json")
    except (SNAPSHOT.SnapshotError, OSError):
        pass
    else:
        raise AssertionError("lstat/open replacement race was accepted")

with tempfile.TemporaryDirectory(prefix="sley-snapshot-pass-") as raw:
    root = Path(raw)
    source = root / "source"
    (source / "nested").mkdir(parents=True)
    (source / "nested" / "main.sley").write_text("task main() -> Int { return 0 }\n", encoding="utf-8")
    os.chmod(source / "nested" / "main.sley", 0o755)
    SNAPSHOT._before_open = lambda _relative: None
    manifest = SNAPSHOT.seal_snapshot(source, root / "sealed", root / "manifest.json")
    file_entry = next(entry for entry in manifest["entries"] if entry["path"] == "nested/main.sley")
    assert file_entry["file_type"] == "regular"
    assert file_entry["mode"] == "100755"
    assert len(file_entry["sha256"]) == 64
    assert (root / "sealed" / "nested" / "main.sley").is_file()

print("sley snapshot adversarial tests passed")
# SLEY_PYTHON_END
PY
