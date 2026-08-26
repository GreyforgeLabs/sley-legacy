#!/usr/bin/env bash
set -euo pipefail

exec python3 /dev/fd/3 "$@" 3<<'PY'
"""Build and verify the supported Sley 1.2 Linux release-candidate archive."""

from __future__ import annotations

import argparse
import datetime as dt
import gzip
import hashlib
import json
import os
import pathlib
import platform
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from typing import Any


TOP_LEVEL_FILES = {
    "CHANGELOG.md",
    "LICENSE",
    "NOTICE",
    "README.md",
    "SLEY_AI.md",
    "Makefile",
    "llms.txt",
}
PAYLOAD_PREFIXES = (
    "assets/",
    "bin/",
    "clients/",
    "docs/",
    "examples/",
    "fixtures/",
    "reports/operational/",
    "scripts/",
    "self-hosted/",
)
EXCLUDED_PATHS = {"docs/SleyDevelopmentPauseCheckpoint.md"}
FORBIDDEN_TEXT = (
    re.compile("/home/" + "greyforge"),
    re.compile(r"[.]codex-worktrees"),
    re.compile(r"[.]local/share/greyforge"),
    re.compile(r"BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY"),
    re.compile(r"ghp_[A-Za-z0-9]{12,}"),
    re.compile(r"sk-[A-Za-z0-9]{12,}"),
    re.compile(r"AKIA[0-9A-Z]{16}"),
)


def fail(message: str) -> None:
    raise SystemExit(f"sley release: {message}")


def run(args: list[str], *, cwd: pathlib.Path, check: bool = True, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, cwd=cwd, check=check, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return f"sha256:{digest.hexdigest()}"


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def write_json(path: pathlib.Path, value: Any) -> None:
    path.write_bytes(canonical_bytes(value))


def env_text(name: str) -> str:
    value = os.environ.get(name, "")
    if not value:
        fail(f"missing internal release value {name}")
    return value


def release_values() -> dict[str, Any]:
    return {
        "version": env_text("SLEY_RELEASE_VERSION"),
        "artifact_id": env_text("SLEY_RELEASE_ARTIFACT_ID"),
        "os": env_text("SLEY_RELEASE_PLATFORM_OS"),
        "architecture": env_text("SLEY_RELEASE_PLATFORM_ARCHITECTURE"),
        "archive_format": env_text("SLEY_RELEASE_ARCHIVE_FORMAT"),
        "support_status": env_text("SLEY_RELEASE_SUPPORT_STATUS"),
        "authority_mode": env_text("SLEY_RELEASE_AUTHORITY_MODE"),
        "manifest_schema": env_text("SLEY_RELEASE_MANIFEST_SCHEMA"),
        "license_schema": env_text("SLEY_RELEASE_LICENSE_SCHEMA"),
        "provenance_schema": env_text("SLEY_RELEASE_PROVENANCE_SCHEMA"),
        "verification_schema": env_text("SLEY_RELEASE_VERIFICATION_SCHEMA"),
        "required_tools": json.loads(env_text("SLEY_RELEASE_REQUIRED_TOOLS_JSON")),
        "entrypoints": json.loads(env_text("SLEY_RELEASE_ENTRYPOINTS_JSON")),
        "metadata_paths": json.loads(env_text("SLEY_RELEASE_METADATA_PATHS_JSON")),
        "publication_authorized": env_text("SLEY_RELEASE_PUBLICATION_AUTHORIZED") == "true",
    }


def ensure_root(path: pathlib.Path) -> pathlib.Path:
    root = path.resolve()
    try:
        git_root = pathlib.Path(run(["git", "rev-parse", "--show-toplevel"], cwd=root).stdout.strip()).resolve()
    except (subprocess.CalledProcessError, OSError):
        fail(f"invalid repository root: {root}")
    if git_root != root:
        fail(f"release root is not the Git worktree root: {root}")
    if not (root / "LICENSE").is_file() or not (root / "docs/schemas").is_dir():
        fail(f"incomplete repository root: {root}")
    return root


def test_authority() -> bool:
    return os.environ.get("SLEY_ALLOW_TEST_HOOKS") == "1" and os.environ.get("SLEY_RELEASE_ALLOW_DIRTY") == "1"


def source_state(root: pathlib.Path, *, enforce_clean: bool = True) -> tuple[str, int, bool]:
    commit = run(["git", "rev-parse", "HEAD"], cwd=root).stdout.strip()
    epoch_text = run(["git", "show", "-s", "--format=%ct", "HEAD"], cwd=root).stdout.strip()
    if not re.fullmatch(r"[0-9a-f]{40}", commit) or not epoch_text.isdigit():
        fail("could not resolve an immutable source commit")
    status_lines = run(["git", "status", "--porcelain=v1", "--untracked-files=all"], cwd=root).stdout.splitlines()
    dirty = any(not line[3:].startswith("dist/") for line in status_lines if len(line) > 3)
    if dirty and enforce_clean and not test_authority():
        fail("source tree is dirty; commit the release inputs before building")
    return commit, int(epoch_text), dirty


def validate_version_sources(root: pathlib.Path, version: str) -> None:
    bootstrap_text = (root / "self-hosted/src/loom/bootstrap.sley").read_text(encoding="utf-8")
    implementation = re.search(r'return "sley ([^"]+)"', bootstrap_text)
    ai_header = re.search(r"^Sley version: ([^\s]+)$", (root / "SLEY_AI.md").read_text(encoding="utf-8"), re.MULTILINE)
    node_version = json.loads((root / "clients/node/package.json").read_text(encoding="utf-8")).get("version")
    python_version = tomllib.loads((root / "clients/python/pyproject.toml").read_text(encoding="utf-8"))["project"]["version"]
    observed = {
        "self-hosted/src/loom/bootstrap.sley": implementation.group(1) if implementation else None,
        "SLEY_AI.md": ai_header.group(1) if ai_header else None,
        "clients/node/package.json": node_version,
        "clients/python/pyproject.toml": python_version,
    }
    mismatched = [path for path, observed_version in observed.items() if observed_version != version]
    if mismatched:
        fail(f"release version mismatch in {mismatched[0]}: expected {version}, found {observed[mismatched[0]]}")


def payload_paths(root: pathlib.Path) -> list[str]:
    raw = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    result: list[str] = []
    for item in raw.split(b"\0"):
        if not item:
            continue
        rel = item.decode("utf-8")
        if rel in EXCLUDED_PATHS:
            continue
        if rel in TOP_LEVEL_FILES or rel.startswith(PAYLOAD_PREFIXES):
            source = root / rel
            if source.is_symlink():
                fail(f"release payload contains a symlink: {rel}")
            if source.is_file():
                result.append(rel)
    return sorted(set(result))


def copy_payload(root: pathlib.Path, stage_root: pathlib.Path, paths: list[str]) -> list[dict[str, Any]]:
    files: list[dict[str, Any]] = []
    for rel in paths:
        source = root / rel
        destination = stage_root / rel
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        executable = bool(source.stat().st_mode & stat.S_IXUSR)
        mode = 0o755 if executable else 0o644
        destination.chmod(mode)
        files.append({
            "path": rel,
            "digest": sha256_file(destination),
            "size": destination.stat().st_size,
            "mode": f"{mode:04o}",
        })
    return files


def payload_digest(files: list[dict[str, Any]]) -> str:
    digest = hashlib.sha256()
    for item in files:
        digest.update(item["path"].encode())
        digest.update(b"\0")
        digest.update(item["digest"].encode())
        digest.update(b"\0")
    return f"sha256:{digest.hexdigest()}"


def license_inventory(values: dict[str, Any], commit: str, stage_root: pathlib.Path) -> dict[str, Any]:
    runtime_dependencies = []
    for spec in values["required_tools"]:
        name, minimum = spec.split("|", 1)
        runtime_dependencies.append({
            "name": name,
            "minimum_version": minimum,
            "bundled": False,
            "license_review_scope": "host_tool_not_redistributed",
        })
    components = [
        {"name": "sley-core", "version": values["version"], "path": "self-hosted", "license_expression": "Apache-2.0", "bundled": True},
        {"name": "sley-worker-client-node", "version": values["version"], "path": "clients/node", "license_expression": "Apache-2.0", "bundled": True},
        {"name": "sley-worker-client-python", "version": values["version"], "path": "clients/python", "license_expression": "Apache-2.0", "bundled": True},
    ]
    return {
        "schema": values["license_schema"],
        "status": "complete",
        "release": values["version"],
        "artifact_id": values["artifact_id"],
        "source_commit": commit,
        "project_license": {"expression": "Apache-2.0", "path": "LICENSE", "digest": sha256_file(stage_root / "LICENSE")},
        "bundled_components": components,
        "runtime_dependencies": runtime_dependencies,
        "authority": {"publication_authorized": False, "third_party_binaries_bundled": False},
    }


def spdx_document(values: dict[str, Any], commit: str, epoch: int, files: list[dict[str, Any]]) -> dict[str, Any]:
    created = dt.datetime.fromtimestamp(epoch, dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    namespace = f"https://sley.dev/spdx/{values['artifact_id']}/{commit}"
    spdx_files = []
    relationships = []
    for index, item in enumerate(files, 1):
        file_id = f"SPDXRef-File-{index}"
        spdx_files.append({
            "SPDXID": file_id,
            "fileName": f"./{item['path']}",
            "checksums": [{"algorithm": "SHA256", "checksumValue": item["digest"].split(":", 1)[1]}],
            "licenseConcluded": "Apache-2.0",
            "copyrightText": "NOASSERTION",
        })
        relationships.append({"spdxElementId": "SPDXRef-Package", "relationshipType": "CONTAINS", "relatedSpdxElement": file_id})
    return {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": values["artifact_id"],
        "documentNamespace": namespace,
        "creationInfo": {"created": created, "creators": ["Tool: sley-release-1.2.0"]},
        "packages": [{
            "name": "sley",
            "SPDXID": "SPDXRef-Package",
            "versionInfo": values["version"],
            "downloadLocation": "NOASSERTION",
            "filesAnalyzed": True,
            "licenseConcluded": "Apache-2.0",
            "licenseDeclared": "Apache-2.0",
            "copyrightText": "NOASSERTION",
        }],
        "files": spdx_files,
        "relationships": [{"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES", "relatedSpdxElement": "SPDXRef-Package"}] + relationships,
    }


def create_archive(stage_parent: pathlib.Path, artifact_id: str, archive: pathlib.Path, epoch: int) -> None:
    temp_archive = archive.with_suffix(archive.suffix + ".tmp")
    with temp_archive.open("wb") as raw_handle:
        with gzip.GzipFile(filename="", mode="wb", compresslevel=9, mtime=epoch, fileobj=raw_handle) as gzip_handle:
            with tarfile.open(fileobj=gzip_handle, mode="w", format=tarfile.PAX_FORMAT) as tar:
                root_info = tarfile.TarInfo(artifact_id)
                root_info.type = tarfile.DIRTYPE
                root_info.mode = 0o755
                root_info.uid = root_info.gid = 0
                root_info.uname = root_info.gname = ""
                root_info.mtime = epoch
                tar.addfile(root_info)
                for source in sorted((stage_parent / artifact_id).rglob("*"), key=lambda path: path.as_posix()):
                    rel = source.relative_to(stage_parent).as_posix()
                    info = tar.gettarinfo(str(source), arcname=rel)
                    info.uid = info.gid = 0
                    info.uname = info.gname = ""
                    info.mtime = epoch
                    if info.isdir():
                        info.mode = 0o755
                        tar.addfile(info)
                    else:
                        info.mode = 0o755 if source.stat().st_mode & stat.S_IXUSR else 0o644
                        with source.open("rb") as handle:
                            tar.addfile(info, handle)
    temp_archive.replace(archive)


def output_paths(output_dir: pathlib.Path, artifact_id: str) -> dict[str, pathlib.Path]:
    archive = output_dir / f"{artifact_id}.tar.gz"
    return {
        "archive": archive,
        "checksum": output_dir / f"{archive.name}.sha256",
        "manifest": output_dir / f"{artifact_id}.manifest.json",
        "licenses": output_dir / f"{artifact_id}.licenses.json",
        "sbom": output_dir / f"{artifact_id}.sbom.spdx.json",
        "provenance": output_dir / f"{artifact_id}.provenance.json",
    }


def build(root: pathlib.Path, output_dir: pathlib.Path, values: dict[str, Any]) -> int:
    validate_version_sources(root, values["version"])
    commit, epoch, dirty = source_state(root)
    status = "test_only" if dirty else "release_candidate"
    output_dir = output_dir.resolve()
    if output_dir == pathlib.Path("/") or output_dir.is_symlink():
        fail("unsafe output directory")
    output_dir.mkdir(parents=True, exist_ok=True)
    outputs = output_paths(output_dir, values["artifact_id"])
    existing = [path for path in outputs.values() if path.exists()]
    replace_allowed = test_authority() and os.environ.get("SLEY_RELEASE_REPLACE") == "1"
    if existing and not replace_allowed:
        fail(f"refusing to overwrite existing release output: {existing[0]}")
    if replace_allowed:
        for path in existing:
            path.unlink()

    paths = payload_paths(root)
    with tempfile.TemporaryDirectory(prefix="sley-release-build-") as temp_text:
        stage_parent = pathlib.Path(temp_text)
        stage_root = stage_parent / values["artifact_id"]
        stage_root.mkdir()
        files = copy_payload(root, stage_root, paths)
        missing_entrypoints = [item for item in values["entrypoints"] if not (stage_root / item).is_file()]
        if missing_entrypoints:
            fail(f"release entrypoint missing from payload: {missing_entrypoints[0]}")
        tree_digest = payload_digest(files)
        manifest = {
            "schema": values["manifest_schema"],
            "status": status,
            "release": values["version"],
            "artifact_id": values["artifact_id"],
            "platform": {
                "os": values["os"],
                "architecture": values["architecture"],
                "archive_format": values["archive_format"],
                "support_status": values["support_status"],
            },
            "source": {"commit": commit, "payload_tree_digest": tree_digest, "dirty": dirty},
            "payload": {
                "inventory_scope": "tracked_release_payload_excluding_release_metadata",
                "file_count": len(files),
                "total_bytes": sum(item["size"] for item in files),
                "files": files,
            },
            "toolchain": {
                "sley_version": f"sley {values['version']}",
                "ai_bootstrap_version": "Bootstrap 0.2",
                "worker_protocol": "sley.worker.v1",
                "required_tools": values["required_tools"],
            },
            "entrypoints": values["entrypoints"],
            "metadata": {"manifest": values["metadata_paths"][0], "license_inventory": values["metadata_paths"][1], "sbom": values["metadata_paths"][2]},
            "authority": {
                "mode": values["authority_mode"],
                "publication_authorized": False,
                "tag_authorized": False,
                "upload_authorized": False,
                "signing_status": "unsigned",
            },
        }
        licenses = license_inventory(values, commit, stage_root)
        sbom = spdx_document(values, commit, epoch, files)
        metadata_dir = stage_root / "release"
        metadata_dir.mkdir()
        write_json(metadata_dir / "manifest.json", manifest)
        write_json(metadata_dir / "licenses.json", licenses)
        write_json(metadata_dir / "sbom.spdx.json", sbom)
        shutil.copyfile(metadata_dir / "manifest.json", outputs["manifest"])
        shutil.copyfile(metadata_dir / "licenses.json", outputs["licenses"])
        shutil.copyfile(metadata_dir / "sbom.spdx.json", outputs["sbom"])
        create_archive(stage_parent, values["artifact_id"], outputs["archive"], epoch)

    archive_digest = sha256_file(outputs["archive"])
    outputs["checksum"].write_text(f"{archive_digest.split(':', 1)[1]}  {outputs['archive'].name}\n", encoding="utf-8")
    provenance = {
        "schema": values["provenance_schema"],
        "status": status,
        "release": values["version"],
        "artifact_id": values["artifact_id"],
        "source": {"commit": commit, "payload_tree_digest": manifest["source"]["payload_tree_digest"], "dirty": dirty},
        "artifact": {"path": outputs["archive"].name, "digest": archive_digest, "size": outputs["archive"].stat().st_size},
        "materials": {
            "manifest_digest": sha256_file(outputs["manifest"]),
            "license_inventory_digest": sha256_file(outputs["licenses"]),
            "sbom_digest": sha256_file(outputs["sbom"]),
        },
        "build": {
            "command": ["bin/sley", "release", "build", "--output-dir", "dist", "--json"],
            "builder_os": platform.system().lower(),
            "builder_architecture": platform.machine(),
            "source_date_epoch": epoch,
            "reproducible": True,
        },
        "assurance": {"signed": False, "signature": None, "cryptographic_provider_attestation": False},
        "authority": {"mode": values["authority_mode"], "publication_authorized": False, "tag_authorized": False, "upload_authorized": False},
    }
    write_json(outputs["provenance"], provenance)
    print(json.dumps(provenance, indent=2, sort_keys=True))
    return 0


def add_check(checks: list[dict[str, str]], check_id: str, passed: bool, message: str) -> None:
    checks.append({"id": check_id, "status": "passed" if passed else "failed", "message": message})


def safe_members(archive: pathlib.Path, artifact_id: str) -> tuple[bool, str]:
    try:
        with tarfile.open(archive, "r:gz") as tar:
            members = tar.getmembers()
    except (tarfile.TarError, OSError) as exc:
        return False, f"archive cannot be read: {exc}"
    if not members:
        return False, "archive is empty"
    prefix = f"{artifact_id}/"
    for member in members:
        path = pathlib.PurePosixPath(member.name)
        if path.is_absolute() or ".." in path.parts:
            return False, f"unsafe archive path: {member.name}"
        if member.name != artifact_id and not member.name.startswith(prefix):
            return False, f"archive member escapes the artifact root: {member.name}"
        if member.issym() or member.islnk() or member.isdev() or member.isfifo():
            return False, f"unsupported archive member type: {member.name}"
    return True, f"{len(members)} archive members are rooted and regular"


def validate_contract(root: pathlib.Path, schema: str, path: pathlib.Path) -> bool:
    result = run(
        [str(root / "bin/sley-contract"), "validate", "--schema-dir", str(root / "docs/schemas"), "--schema", schema, str(path), "--json"],
        cwd=root,
        check=False,
    )
    return result.returncode == 0


def verify(root: pathlib.Path, output_dir: pathlib.Path, values: dict[str, Any], skip_client_tests: bool) -> int:
    if skip_client_tests and not test_authority():
        fail("--skip-client-tests is available only to the bounded test harness")
    output_dir = output_dir.resolve()
    current_commit, _, current_dirty = source_state(root, enforce_clean=False)
    outputs = output_paths(output_dir, values["artifact_id"])
    checks: list[dict[str, str]] = []
    issues: list[dict[str, str]] = []
    required_present = all(path.is_file() and not path.is_symlink() for path in outputs.values())
    add_check(checks, "ARTIFACT_SET_PRESENT", required_present, "all release artifacts are present" if required_present else "one or more release artifacts are missing")
    provenance: dict[str, Any] = {}
    manifest: dict[str, Any] = {}
    if required_present:
        try:
            provenance = json.loads(outputs["provenance"].read_text())
            manifest = json.loads(outputs["manifest"].read_text())
        except (json.JSONDecodeError, OSError) as exc:
            issues.append({"code": "RELEASE_METADATA_INVALID", "message": str(exc)})
    archive_safe, archive_safe_message = safe_members(outputs["archive"], values["artifact_id"]) if outputs["archive"].is_file() else (False, "archive is missing")
    add_check(checks, "ARCHIVE_PATH_SAFE", archive_safe, archive_safe_message)

    digest_match = False
    if outputs["archive"].is_file() and outputs["checksum"].is_file() and provenance:
        actual_digest = sha256_file(outputs["archive"])
        checksum_parts = outputs["checksum"].read_text().strip().split()
        digest_match = (
            len(checksum_parts) == 2
            and checksum_parts[0] == actual_digest.split(":", 1)[1]
            and checksum_parts[1] == outputs["archive"].name
            and provenance.get("artifact", {}).get("digest") == actual_digest
            and provenance.get("artifact", {}).get("size") == outputs["archive"].stat().st_size
        )
    add_check(checks, "ARCHIVE_DIGEST_MATCH", digest_match, "archive checksum and provenance agree" if digest_match else "archive checksum or provenance does not match")

    contracts_ok = bool(provenance and manifest)
    if contracts_ok:
        for schema, path in (
            (values["manifest_schema"], outputs["manifest"]),
            (values["license_schema"], outputs["licenses"]),
            (values["provenance_schema"], outputs["provenance"]),
        ):
            contracts_ok = contracts_ok and validate_contract(root, schema, path)
    add_check(checks, "RELEASE_CONTRACTS_VALID", contracts_ok, "release JSON contracts validate" if contracts_ok else "one or more release JSON contracts are invalid")

    source_commit_match = (
        bool(manifest)
        and manifest.get("source", {}).get("commit") == current_commit
        and provenance.get("source", {}).get("commit") == current_commit
        and manifest.get("source", {}).get("commit") == provenance.get("source", {}).get("commit")
    )
    add_check(checks, "SOURCE_COMMIT_MATCH", source_commit_match, "artifact source commit matches the current checkout HEAD" if source_commit_match else "artifact source commit does not match the current checkout HEAD")

    source_clean = bool(manifest) and not current_dirty and manifest.get("source", {}).get("dirty") is False and provenance.get("source", {}).get("dirty") is False
    if test_authority():
        source_clean = bool(manifest) and manifest.get("source", {}).get("dirty") in (True, False) and provenance.get("source", {}).get("dirty") in (True, False)
    add_check(checks, "SOURCE_STATE_ALLOWED", source_clean, "source state is permitted for this verification mode" if source_clean else "release candidate was built from dirty source")

    metadata_ok = False
    payload_ok = False
    scrub_ok = False
    smoke_ok = False
    client_ok = skip_client_tests
    if archive_safe and digest_match and contracts_ok and source_commit_match and source_clean:
        with tempfile.TemporaryDirectory(prefix="sley-release-verify-") as temp_text:
            extract_parent = pathlib.Path(temp_text)
            with tarfile.open(outputs["archive"], "r:gz") as tar:
                tar.extractall(extract_parent, filter="data")
            unpacked = extract_parent / values["artifact_id"]
            metadata_ok = all(
                (unpacked / internal).read_bytes() == external.read_bytes()
                for internal, external in (
                    ("release/manifest.json", outputs["manifest"]),
                    ("release/licenses.json", outputs["licenses"]),
                    ("release/sbom.spdx.json", outputs["sbom"]),
                )
            )
            if provenance:
                metadata_ok = metadata_ok and all(
                    provenance.get("materials", {}).get(field) == sha256_file(path)
                    for field, path in (
                        ("manifest_digest", outputs["manifest"]),
                        ("license_inventory_digest", outputs["licenses"]),
                        ("sbom_digest", outputs["sbom"]),
                    )
                )
            if manifest:
                actual_files: list[dict[str, Any]] = []
                for expected in manifest.get("payload", {}).get("files", []):
                    target = unpacked / expected.get("path", "")
                    if not target.is_file() or target.is_symlink():
                        break
                    mode = "0755" if target.stat().st_mode & stat.S_IXUSR else "0644"
                    actual_files.append({"path": expected["path"], "digest": sha256_file(target), "size": target.stat().st_size, "mode": mode})
                payload_ok = (
                    actual_files == manifest.get("payload", {}).get("files", [])
                    and payload_digest(actual_files) == manifest.get("source", {}).get("payload_tree_digest")
                    and len(actual_files) == manifest.get("payload", {}).get("file_count")
                    and sum(item["size"] for item in actual_files) == manifest.get("payload", {}).get("total_bytes")
                )
            scrub_ok = True
            for path in unpacked.rglob("*"):
                if not path.is_file():
                    continue
                text = path.read_text(encoding="utf-8", errors="ignore")
                if any(pattern.search(text) for pattern in FORBIDDEN_TEXT):
                    scrub_ok = False
                    break
            smoke_env = os.environ.copy()
            smoke_env["PATH"] = f"{unpacked / 'bin'}:{smoke_env.get('PATH', '')}"
            smoke_commands = (
                ([str(unpacked / "bin/sley"), "--version"], "sley 1.2.0"),
                ([str(unpacked / "bin/sley"), "doctor", "--toolchain", "--json"], '"status": "ready"'),
                ([str(unpacked / "bin/sley"), "check", "--json", "examples/hello.sley"], '"status": "ok"'),
            )
            smoke_ok = True
            for command, needle in smoke_commands:
                result = run(command, cwd=unpacked, check=False, env=smoke_env)
                if result.returncode != 0 or needle not in result.stdout:
                    smoke_ok = False
                    break
            if smoke_ok:
                result = run(["make", "build-cli", "build-bins", "contracts"], cwd=unpacked, check=False, env=smoke_env)
                smoke_ok = result.returncode == 0
            if not skip_client_tests:
                result = run(["make", "worker-clients"], cwd=unpacked, check=False, env=smoke_env)
                client_ok = result.returncode == 0

    add_check(checks, "RELEASE_METADATA_BOUND", metadata_ok, "embedded and external metadata are digest-bound" if metadata_ok else "release metadata binding failed")
    add_check(checks, "PAYLOAD_INVENTORY_MATCH", payload_ok, "payload matches the manifest inventory" if payload_ok else "payload differs from the manifest inventory")
    add_check(checks, "PAYLOAD_SCRUB_PASSED", scrub_ok, "payload contains no forbidden host paths or secret patterns" if scrub_ok else "payload scrub found forbidden content")
    add_check(checks, "UNPACKED_TOOLCHAIN_SMOKE", smoke_ok, "unpacked CLI, doctor, checker, and contracts passed" if smoke_ok else "unpacked toolchain smoke failed")
    add_check(checks, "WORKER_CLIENT_COMPATIBILITY", client_ok, "worker clients passed or were explicitly skipped by the test harness" if client_ok else "worker client compatibility failed")

    failed = [item for item in checks if item["status"] == "failed"]
    issues.extend({"code": item["id"], "message": item["message"]} for item in failed)
    report = {
        "schema": values["verification_schema"],
        "status": "failed" if failed else "passed",
        "release": values["version"],
        "artifact_id": values["artifact_id"],
        "archive": outputs["archive"].name,
        "source_commit": provenance.get("source", {}).get("commit"),
        "check_count": len(checks),
        "passed_count": len(checks) - len(failed),
        "failed_count": len(failed),
        "checks": checks,
        "issues": issues,
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    return 1 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser(prog="sley release")
    parser.add_argument("--repo-root", required=True, type=pathlib.Path, help=argparse.SUPPRESS)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for name in ("build", "verify"):
        command = subparsers.add_parser(name)
        command.add_argument("--output-dir", default="dist", type=pathlib.Path)
        command.add_argument("--json", action="store_true")
        if name == "verify":
            command.add_argument("--skip-client-tests", action="store_true")
    args = parser.parse_args()
    root = ensure_root(args.repo_root)
    output_dir = args.output_dir if args.output_dir.is_absolute() else root / args.output_dir
    values = release_values()
    if values["publication_authorized"]:
        fail("release vocabulary unexpectedly grants publication authority")
    if args.command == "build":
        return build(root, output_dir, values)
    return verify(root, output_dir, values, args.skip_client_tests)


if __name__ == "__main__":
    raise SystemExit(main())
PY
