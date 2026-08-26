#!/usr/bin/env python3
"""Generate worker client models and constants from canonical JSON Schemas."""

from __future__ import annotations

import argparse
import json
import keyword
import re
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_IDS = (
    "sley.worker.request.v1",
    "sley.worker.response.v1",
    "sley.worker.event.v1",
    "sley.worker.session.v1",
)
ROOT_NAMES = {
    "sley.worker.request.v1": "WorkerRequest",
    "sley.worker.response.v1": "WorkerResponse",
    "sley.worker.event.v1": "WorkerEvent",
    "sley.worker.session.v1": "WorkerSession",
}
OUTPUTS = {
    "python": ROOT / "clients/python/sley_worker_client/generated/models.py",
    "node_types": ROOT / "clients/node/generated/models.d.ts",
    "node_runtime": ROOT / "clients/node/generated/constants.mjs",
}


def camel(value: str) -> str:
    return "".join(part[:1].upper() + part[1:] for part in re.split(r"[^A-Za-z0-9]+", value) if part)


def literal(value: Any, language: str) -> str:
    if language == "python":
        if value is None:
            return "None"
        if value is True:
            return "True"
        if value is False:
            return "False"
        return repr(value)
    return json.dumps(value, ensure_ascii=False)


class ModelRegistry:
    def __init__(self, schemas: dict[str, dict[str, Any]]):
        self.schemas = schemas
        self.declarations: list[tuple[str, dict[str, Any], str]] = []
        self.names_by_object: dict[int, str] = {}
        self.names_by_ref: dict[tuple[str, str], str] = {}
        for schema_id in SCHEMA_IDS:
            schema = schemas[schema_id]
            root_name = ROOT_NAMES[schema_id]
            self._register(root_name, schema, schema_id)
            self.names_by_ref[(schema_id, "")] = root_name
            for key, definition in schema.get("$defs", {}).items():
                name = root_name + camel(key)
                self._register(name, definition, schema_id)
                self.names_by_ref[(schema_id, f"/$defs/{key}")] = name

    def _register(self, name: str, schema: dict[str, Any], schema_id: str) -> None:
        if id(schema) in self.names_by_object:
            return
        self.names_by_object[id(schema)] = name
        self.declarations.append((name, schema, schema_id))
        self._discover_nested(name, schema, schema_id)

    def _discover_nested(self, parent: str, schema: dict[str, Any], schema_id: str) -> None:
        for property_name, child in schema.get("properties", {}).items():
            if not isinstance(child, dict):
                continue
            self._discover_child(parent + camel(property_name), child, schema_id)

    def _discover_child(self, name: str, schema: dict[str, Any], schema_id: str) -> None:
        if schema.get("type") == "object" and schema.get("properties"):
            self._register(name, schema, schema_id)
            return
        if schema.get("type") == "array" and isinstance(schema.get("items"), dict):
            self._discover_child(name + "Item", schema["items"], schema_id)
        for index, choice in enumerate(schema.get("oneOf", [])):
            if isinstance(choice, dict):
                self._discover_child(name + "Choice" + str(index + 1), choice, schema_id)

    def resolve_ref(self, ref: str, current_schema_id: str) -> str:
        if ref.startswith("#"):
            schema_id = current_schema_id
            fragment = ref[1:]
        else:
            schema_id, separator, fragment = ref.partition("#")
            if not separator:
                fragment = ""
        key = (schema_id, fragment)
        if key not in self.names_by_ref:
            raise ValueError(f"unsupported worker schema reference: {ref}")
        return self.names_by_ref[key]

    def type_expression(self, schema: dict[str, Any], schema_id: str, language: str) -> str:
        if "$ref" in schema:
            return self.resolve_ref(schema["$ref"], schema_id)
        if "const" in schema:
            value = literal(schema["const"], language)
            return f"Literal[{value}]" if language == "python" else value
        if "enum" in schema:
            values = [literal(value, language) for value in schema["enum"]]
            if language == "python":
                return "Literal[" + ", ".join(values) + "]"
            return " | ".join(values)
        if "oneOf" in schema:
            values = [self.type_expression(choice, schema_id, language) for choice in schema["oneOf"]]
            values = list(dict.fromkeys(values))
            separator = " | "
            return separator.join(values)
        schema_type = schema.get("type")
        if schema_type == "string":
            return "str" if language == "python" else "string"
        if schema_type == "integer":
            return "int" if language == "python" else "number"
        if schema_type == "boolean":
            return "bool" if language == "python" else "boolean"
        if schema_type == "null":
            return "None" if language == "python" else "null"
        if schema_type == "array":
            item = self.type_expression(schema.get("items", {}), schema_id, language)
            return f"list[{item}]" if language == "python" else f"ReadonlyArray<{item}>"
        if schema_type == "object":
            declared = self.names_by_object.get(id(schema))
            if declared is not None:
                return declared
            return "dict[str, JsonValue]" if language == "python" else "Readonly<Record<string, JsonValue>>"
        return "JsonValue"


def load_schemas() -> dict[str, dict[str, Any]]:
    schemas: dict[str, dict[str, Any]] = {}
    for schema_id in SCHEMA_IDS:
        path = ROOT / "docs/schemas" / f"{schema_id}.schema.json"
        value = json.loads(path.read_text(encoding="utf-8"))
        if value.get("$id") != schema_id:
            raise ValueError(f"schema identity mismatch in {path}")
        schemas[schema_id] = value
    return schemas


def contract_versions(schemas: dict[str, dict[str, Any]]) -> list[str]:
    request = schemas["sley.worker.request.v1"]
    versions = request["$defs"]["bindings"]["properties"]["contract_versions"]["items"]["enum"]
    if len(versions) != request["$defs"]["bindings"]["properties"]["contract_versions"]["minItems"]:
        raise ValueError("worker contract version inventory is not exact")
    return versions


def operations(schemas: dict[str, dict[str, Any]]) -> list[str]:
    return schemas["sley.worker.request.v1"]["properties"]["operation"]["enum"]


def protocol(schemas: dict[str, dict[str, Any]]) -> str:
    values = {schema["properties"]["protocol"]["const"] for schema in schemas.values()}
    if len(values) != 1:
        raise ValueError("worker schemas disagree on protocol identity")
    return values.pop()


def render_python(registry: ModelRegistry, schemas: dict[str, dict[str, Any]]) -> str:
    lines = [
        "# Generated by clients/generate-worker-clients.py. Do not edit.",
        "from __future__ import annotations",
        "",
        "from typing import Literal, NotRequired, TypeAlias, TypedDict",
        "",
        "JsonScalar: TypeAlias = str | int | float | bool | None",
        "JsonValue: TypeAlias = JsonScalar | list[\"JsonValue\"] | dict[str, \"JsonValue\"]",
        "",
        f"PROTOCOL = {protocol(schemas)!r}",
        f"SCHEMA_IDS = {tuple(SCHEMA_IDS)!r}",
        f"CONTRACT_VERSIONS = {tuple(contract_versions(schemas))!r}",
        f"OPERATIONS = {tuple(operations(schemas))!r}",
        "ZERO_DIGEST = \"sha256:\" + (\"0\" * 64)",
        "",
    ]
    for name, schema, schema_id in registry.declarations:
        if schema.get("type") != "object":
            lines.append(f"{name}: TypeAlias = {registry.type_expression(schema, schema_id, 'python')}")
            lines.append("")
            continue
        required = set(schema.get("required", []))
        properties = schema.get("properties", {})
        if any(not field.isidentifier() or keyword.iskeyword(field) for field in properties):
            lines.append(f"{name} = TypedDict({name!r}, {{")
            for field, field_schema in properties.items():
                annotation = registry.type_expression(field_schema, schema_id, "python")
                if field not in required:
                    annotation = f"NotRequired[{annotation}]"
                lines.append(f"    {field!r}: {annotation},")
            lines.append("})")
            lines.append("")
            continue
        lines.append(f"class {name}(TypedDict):")
        if not properties:
            lines.append("    pass")
        for field, field_schema in properties.items():
            annotation = registry.type_expression(field_schema, schema_id, "python")
            if field not in required:
                annotation = f"NotRequired[{annotation}]"
            lines.append(f"    {field}: {annotation}")
        lines.append("")
    exports = [name for name, _, _ in registry.declarations]
    exports.extend(["JsonScalar", "JsonValue", "PROTOCOL", "SCHEMA_IDS", "CONTRACT_VERSIONS", "OPERATIONS", "ZERO_DIGEST"])
    lines.append("__all__ = [")
    lines.extend(f"    {name!r}," for name in sorted(exports))
    lines.append("]")
    lines.append("")
    return "\n".join(lines)


def render_node_types(registry: ModelRegistry, schemas: dict[str, dict[str, Any]]) -> str:
    lines = [
        "// Generated by clients/generate-worker-clients.py. Do not edit.",
        "export type JsonScalar = string | number | boolean | null;",
        "export type JsonValue = JsonScalar | ReadonlyArray<JsonValue> | { readonly [key: string]: JsonValue };",
        "",
        f"export declare const PROTOCOL: {json.dumps(protocol(schemas))};",
        "export declare const SCHEMA_IDS: readonly [" + ", ".join(json.dumps(item) for item in SCHEMA_IDS) + "];",
        "export declare const CONTRACT_VERSIONS: readonly [" + ", ".join(json.dumps(item) for item in contract_versions(schemas)) + "];",
        "export declare const OPERATIONS: readonly [" + ", ".join(json.dumps(item) for item in operations(schemas)) + "];",
        "export declare const ZERO_DIGEST: `sha256:${string}`;",
        "",
    ]
    for name, schema, schema_id in registry.declarations:
        if schema.get("type") != "object":
            lines.append(f"export type {name} = {registry.type_expression(schema, schema_id, 'typescript')};")
            lines.append("")
            continue
        required = set(schema.get("required", []))
        lines.append(f"export type {name} = Readonly<{{")
        for field, field_schema in schema.get("properties", {}).items():
            annotation = registry.type_expression(field_schema, schema_id, "typescript")
            optional = "" if field in required else "?"
            lines.append(f"  readonly {field}{optional}: {annotation};")
        lines.append("}>;")
        lines.append("")
    return "\n".join(lines)


def render_node_runtime(schemas: dict[str, dict[str, Any]]) -> str:
    return "\n".join([
        "// Generated by clients/generate-worker-clients.py. Do not edit.",
        f"export const PROTOCOL = Object.freeze({json.dumps(protocol(schemas))});",
        "export const SCHEMA_IDS = Object.freeze(" + json.dumps(list(SCHEMA_IDS), separators=(",", ":")) + ");",
        "export const CONTRACT_VERSIONS = Object.freeze(" + json.dumps(contract_versions(schemas), separators=(",", ":")) + ");",
        "export const OPERATIONS = Object.freeze(" + json.dumps(operations(schemas), separators=(",", ":")) + ");",
        "export const ZERO_DIGEST = \"sha256:\" + \"0\".repeat(64);",
        "",
    ])


def compare_or_write(path: Path, content: str, check: bool) -> bool:
    if check:
        if not path.exists() or path.read_text(encoding="utf-8") != content:
            print(f"generated worker client drift: {path.relative_to(ROOT)}", file=sys.stderr)
            return False
        return True
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    print(path.relative_to(ROOT))
    return True


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    schemas = load_schemas()
    registry = ModelRegistry(schemas)
    rendered = {
        "python": render_python(registry, schemas),
        "node_types": render_node_types(registry, schemas),
        "node_runtime": render_node_runtime(schemas),
    }
    return 0 if all(compare_or_write(OUTPUTS[key], rendered[key], args.check) for key in OUTPUTS) else 1


if __name__ == "__main__":
    raise SystemExit(main())
