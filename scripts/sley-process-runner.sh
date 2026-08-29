#!/usr/bin/env bash
set -euo pipefail

exec python3 /dev/fd/3 "$@" 3<<'PY'
# SLEY_PYTHON_BEGIN
"""Run one bounded process group and reap every adopted descendant."""

from __future__ import annotations

import argparse
import ctypes
import os
import resource
import signal
import subprocess
import sys
import time
from pathlib import Path


PR_SET_CHILD_SUBREAPER = 36


def enable_subreaper() -> None:
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) != 0:
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error))


def atomic_text(path: Path, value: str) -> None:
    temporary = path.with_name(path.name + f".tmp.{os.getpid()}")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600)
    try:
        os.write(descriptor, value.encode("ascii"))
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    os.replace(temporary, path)


def signal_group(pid: int, requested: signal.Signals) -> None:
    try:
        os.killpg(pid, requested)
    except ProcessLookupError:
        pass


def reap_adopted(pgid: int, timeout: float = 2.0) -> None:
    deadline = time.monotonic() + timeout
    while True:
        reaped_any = False
        while True:
            try:
                child, _status = os.waitpid(-1, os.WNOHANG)
            except ChildProcessError:
                return
            if child == 0:
                break
            reaped_any = True
        if time.monotonic() >= deadline:
            signal_group(pgid, signal.SIGKILL)
        if not reaped_any:
            time.sleep(0.01)
        if time.monotonic() >= deadline + 1:
            raise RuntimeError("descendant processes did not reach a reapable state")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--timeout-seconds", type=int, required=True)
    parser.add_argument("--file-limit-bytes", type=int, required=True)
    parser.add_argument("--cancel-file", type=Path, required=True)
    parser.add_argument("--pgid-file", type=Path, required=True)
    parser.add_argument("--stdout-file", type=Path, required=True)
    parser.add_argument("--stderr-file", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("a command is required after --")
    if args.cancel_file.exists():
        return 130

    enable_subreaper()
    stdout_fd = os.open(args.stdout_file, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600)
    stderr_fd = os.open(args.stderr_file, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600)

    def set_limits() -> None:
        resource.setrlimit(resource.RLIMIT_FSIZE, (args.file_limit_bytes, args.file_limit_bytes))

    try:
        process = subprocess.Popen(
            command,
            stdin=subprocess.DEVNULL,
            stdout=stdout_fd,
            stderr=stderr_fd,
            close_fds=True,
            start_new_session=True,
            preexec_fn=set_limits,
        )
    finally:
        os.close(stdout_fd)
        os.close(stderr_fd)

    atomic_text(args.pgid_file, f"{process.pid}\n")
    deadline = time.monotonic() + args.timeout_seconds
    outcome = "normal"
    while process.poll() is None:
        if args.cancel_file.exists():
            outcome = "cancelled"
            signal_group(process.pid, signal.SIGTERM)
            break
        if time.monotonic() >= deadline:
            outcome = "timeout"
            signal_group(process.pid, signal.SIGTERM)
            break
        time.sleep(0.01)

    if process.poll() is None:
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            signal_group(process.pid, signal.SIGKILL)
            process.wait(timeout=2)
    return_code = process.returncode
    signal_group(process.pid, signal.SIGKILL)
    reap_adopted(process.pid)

    if outcome == "cancelled":
        return 130
    if outcome == "timeout":
        return 124
    if return_code is None:
        return 125
    if return_code < 0:
        return 128 + min(-return_code, 127)
    return min(return_code, 255)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"sley_process_runner: error: {exc}", file=sys.stderr)
        raise SystemExit(125)
# SLEY_PYTHON_END
PY
