#!/usr/bin/env bash
set -euo pipefail

exec python3 /dev/fd/3 "$@" 3<<'PY'
# SLEY_PYTHON_BEGIN
"""Seal a directory tree without following mutable or special entries."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import sys
from pathlib import Path
from typing import Any, Callable


class SnapshotError(RuntimeError):
    """Raised when a source tree cannot be copied without changing meaning."""


# Tests replace this callback to force a path swap between lstat and open.
_before_open: Callable[[str], None] = lambda _path: None


def _entry_kind(mode: int) -> str:
    if stat.S_ISLNK(mode):
        return "symlink"
    if stat.S_ISDIR(mode):
        return "directory"
    if stat.S_ISREG(mode):
        return "regular file"
    if stat.S_ISFIFO(mode):
        return "FIFO"
    if stat.S_ISSOCK(mode):
        return "socket"
    if stat.S_ISCHR(mode):
        return "character device"
    if stat.S_ISBLK(mode):
        return "block device"
    return "special file"


def _same_identity(before: os.stat_result, after: os.stat_result) -> bool:
    return (
        before.st_dev,
        before.st_ino,
        before.st_mode,
        before.st_size,
        before.st_mtime_ns,
        before.st_ctime_ns,
    ) == (
        after.st_dev,
        after.st_ino,
        after.st_mode,
        after.st_size,
        after.st_mtime_ns,
        after.st_ctime_ns,
    )


def _display(parts: tuple[str, ...]) -> str:
    return "/".join(parts) or "."


def _copy_regular(
    source_fd: int,
    destination_fd: int,
    name: str,
    relative: tuple[str, ...],
    before: os.stat_result,
) -> dict[str, Any]:
    display = _display(relative)
    _before_open(display)
    flags = os.O_RDONLY | os.O_CLOEXEC
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        input_fd = os.open(name, flags, dir_fd=source_fd)
    except OSError as exc:
        raise SnapshotError(f"could not open snapshot entry without following it: {display}: {exc}") from exc

    output_fd = -1
    try:
        opened = os.fstat(input_fd)
        if not stat.S_ISREG(opened.st_mode) or not _same_identity(before, opened):
            raise SnapshotError(f"snapshot entry changed before it could be opened safely: {display}")
        mode = 0o755 if opened.st_mode & 0o111 else 0o644
        output_fd = os.open(
            name,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC,
            mode,
            dir_fd=destination_fd,
        )
        digest = hashlib.sha256()
        size = 0
        while True:
            chunk = os.read(input_fd, 1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
            size += len(chunk)
            view = memoryview(chunk)
            while view:
                written = os.write(output_fd, view)
                view = view[written:]
        os.fsync(output_fd)
        finished = os.fstat(input_fd)
        current = os.stat(name, dir_fd=source_fd, follow_symlinks=False)
        if not _same_identity(opened, finished) or not _same_identity(finished, current):
            raise SnapshotError(f"snapshot entry changed while it was being copied: {display}")
        if size != opened.st_size:
            raise SnapshotError(f"snapshot entry size changed while it was being copied: {display}")
        return {
            "path": display,
            "file_type": "regular",
            "mode": "100755" if mode == 0o755 else "100644",
            "size": size,
            "sha256": digest.hexdigest(),
        }
    finally:
        if output_fd >= 0:
            os.close(output_fd)
        os.close(input_fd)


def _walk(
    source_fd: int,
    destination_fd: int,
    parts: tuple[str, ...],
    entries: list[dict[str, Any]],
) -> None:
    try:
        names = sorted(os.listdir(source_fd))
    except OSError as exc:
        raise SnapshotError(f"could not list snapshot directory {_display(parts)}: {exc}") from exc

    for name in names:
        if name in (".", "..") or "/" in name or "\0" in name:
            raise SnapshotError(f"invalid snapshot entry name under {_display(parts)}")
        relative = (*parts, name)
        display = _display(relative)
        try:
            before = os.stat(name, dir_fd=source_fd, follow_symlinks=False)
        except OSError as exc:
            raise SnapshotError(f"could not inspect snapshot entry {display}: {exc}") from exc

        if stat.S_ISREG(before.st_mode):
            entries.append(_copy_regular(source_fd, destination_fd, name, relative, before))
            continue
        if not stat.S_ISDIR(before.st_mode):
            raise SnapshotError(f"snapshot entry is a {_entry_kind(before.st_mode)}: {display}")

        _before_open(display)
        flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC
        if hasattr(os, "O_NOFOLLOW"):
            flags |= os.O_NOFOLLOW
        try:
            child_source_fd = os.open(name, flags, dir_fd=source_fd)
        except OSError as exc:
            raise SnapshotError(f"could not open snapshot directory without following it: {display}: {exc}") from exc
        child_destination_fd = -1
        try:
            opened = os.fstat(child_source_fd)
            if not stat.S_ISDIR(opened.st_mode) or not _same_identity(before, opened):
                raise SnapshotError(f"snapshot directory changed before it could be opened safely: {display}")
            os.mkdir(name, 0o755, dir_fd=destination_fd)
            child_destination_fd = os.open(
                name,
                os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC,
                dir_fd=destination_fd,
            )
            entries.append({"path": display, "file_type": "directory", "mode": "040755"})
            _walk(child_source_fd, child_destination_fd, relative, entries)
            current = os.stat(name, dir_fd=source_fd, follow_symlinks=False)
            if not _same_identity(opened, current):
                raise SnapshotError(f"snapshot directory changed while it was being copied: {display}")
        finally:
            if child_destination_fd >= 0:
                os.close(child_destination_fd)
            os.close(child_source_fd)


def seal_snapshot(source: Path, destination: Path, manifest: Path) -> dict[str, Any]:
    """Copy ``source`` to a new ``destination`` after sealing every entry."""

    if destination.exists() or destination.is_symlink():
        raise SnapshotError(f"snapshot destination already exists: {destination}")
    if manifest.exists() or manifest.is_symlink():
        raise SnapshotError(f"snapshot manifest already exists: {manifest}")
    destination.mkdir(mode=0o700, parents=False)
    source_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC
    if hasattr(os, "O_NOFOLLOW"):
        source_flags |= os.O_NOFOLLOW
    source_fd = os.open(source, source_flags)
    destination_fd = os.open(destination, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
    entries: list[dict[str, Any]] = []
    try:
        _walk(source_fd, destination_fd, (), entries)
    finally:
        os.close(destination_fd)
        os.close(source_fd)

    value = {
        "schema": "sley.sealed-snapshot.v1",
        "root_type": "directory",
        "entries": entries,
    }
    manifest.write_text(
        json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    os.chmod(manifest, 0o600)
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    try:
        seal_snapshot(args.source, args.destination, args.manifest)
    except (OSError, SnapshotError) as exc:
        print(f"sley_snapshot: error: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
# SLEY_PYTHON_END
PY
