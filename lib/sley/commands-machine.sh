# shellcheck shell=bash
# Bounded machine protocol runtime and arena delegation.

command_machine() {
  local target="" package_id="" package_version="" protocol_version="sley.machine.invoke.v0" files_json runtime_digest check_report
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --json)
        shift
        ;;
      --source|--target)
        target="$2"
        shift 2
        ;;
      --package-id)
        package_id="$2"
        shift 2
        ;;
      --package-version)
        package_version="$2"
        shift 2
        ;;
      --protocol-version)
        protocol_version="$2"
        shift 2
        ;;
      --*)
        echo "unknown sley machine option: $1" >&2
        return 64
        ;;
      *)
        target="$1"
        shift
        ;;
    esac
  done
  if [[ -z "$target" || -z "$package_id" || -z "$package_version" ]]; then
    echo "usage: sley machine --source <file-or-project> --package-id <id> --package-version <version>" >&2
    return 64
  fi
  files_json="$(collect_files "$target" | jq -R . | jq -s .)"
  runtime_digest="$(sha256sum "$SELF_PATH" | awk '{print "sha256:" $1}')"
  check_report="$(check_json "$target")"
  SLEY_MACHINE_FILES_JSON="$files_json" \
  SLEY_MACHINE_CHECK_REPORT="$check_report" \
  python3 /dev/fd/3 "$ROOT_DIR" "$target" "$package_id" "$package_version" "$VERSION" "$runtime_digest" "$protocol_version" 3<<'PY'
import hashlib
import json
import os
import re
import signal
import subprocess
import sys
from pathlib import Path

ROOT = Path(sys.argv[1]).resolve()
TARGET = sys.argv[2]
PACKAGE_ID = sys.argv[3]
PACKAGE_VERSION = sys.argv[4]
RUNTIME_VERSION = sys.argv[5]
RUNTIME_DIGEST = sys.argv[6]
PROTOCOL_VERSION = sys.argv[7]
SCHEMA = "sley.machine.response.v0"
RUNTIME_ID = "sley.stage1.machine"
CHECK_REPORT = json.loads(os.environ.get("SLEY_MACHINE_CHECK_REPORT", "{}"))
CHECK_PASSED = CHECK_REPORT.get("status") == "ok"

DEFAULT_LIMITS = {
    "requestLineBytes": 4096,
    "responseBytes": 262144,
    "batch": 500,
    "evaluationSteps": 250000,
    "callDepth": 128,
    "collectionLength": 4096,
    "timeoutMs": 2000,
}

REQUEST_FIELDS = {
    "protocolVersion",
    "requestId",
    "operation",
    "packageId",
    "packageVersion",
    "task",
    "input",
    "limits",
    "pins",
}
LIMIT_FIELDS = set(DEFAULT_LIMITS)
PIN_FIELDS = {"sourceDigest", "runtimeId", "runtimeVersion", "runtimeDigest"}


class MachineError(Exception):
    def __init__(
        self,
        code,
        message,
        *,
        exit_code=70,
        failure_class="runtime",
        phase="execution",
        path="",
        limit=None,
        counters=None,
    ):
        super().__init__(message)
        self.code = code
        self.message = message
        self.exit_code = exit_code
        self.failure_class = failure_class
        self.phase = phase
        self.path = path
        self.limit = limit
        self.counters = counters


class ReturnValue(Exception):
    def __init__(self, value):
        super().__init__("return")
        self.value = value


def canonical_json(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)


def digest_bytes(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def digest_json(value):
    return digest_bytes(canonical_json(value).encode("utf-8"))


def load_files():
    files = [Path(path).resolve() for path in json.loads(os.environ.get("SLEY_MACHINE_FILES_JSON", "[]"))]
    if not files:
        raise MachineError(
            "SLEY_MACHINE_SOURCE_EMPTY",
            "source contains no Sley files",
            exit_code=70,
            failure_class="runtime",
            phase="input",
            path="/source",
        )
    return sorted(files, key=lambda path: str(path))


SOURCE_FILES = load_files()
target_path = Path(TARGET)
SOURCE_ROOT = (target_path if target_path.is_dir() else target_path.parent).resolve()


def source_digest():
    hasher = hashlib.sha256()
    for path in SOURCE_FILES:
        try:
            rel = path.relative_to(SOURCE_ROOT).as_posix()
        except ValueError:
            rel = path.name
        hasher.update(rel.encode("utf-8"))
        hasher.update(b"\0")
        hasher.update(path.read_bytes())
        hasher.update(b"\0")
    return "sha256:" + hasher.hexdigest()


SOURCE_DIGEST = source_digest()
SOURCE_INFO = {
    "digest": SOURCE_DIGEST,
    "fileCount": len(SOURCE_FILES),
}
RUNTIME_INFO = {
    "id": RUNTIME_ID,
    "version": RUNTIME_VERSION,
    "digest": RUNTIME_DIGEST,
}
PACKAGE_INFO = {
    "id": PACKAGE_ID,
    "version": PACKAGE_VERSION,
}


class NodeTextRuntime:
    def __init__(self):
        source = r'''
const readline = require("node:readline");
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on("line", (line) => {
  const request = JSON.parse(line);
  const value = request.value;
  let result;
  if (request.op === "nfkc_trim") {
    result = value.normalize("NFKC").trim();
  } else if (request.op === "ascii_fold_en") {
    result = [...value.normalize("NFKD").replace(/[\u0300-\u036f]/gu, "").toLocaleUpperCase("en")]
      .filter((character) => character >= "A" && character <= "Z")
      .join("");
  } else if (request.op === "utf16_length") {
    result = value.length;
  } else if (request.op === "has_prohibited_controls") {
    result = /[\u0000-\u001f\u007f\u200b-\u200f\u202a-\u202e\u2066-\u2069]/u.test(value);
  } else {
    throw new Error("unknown text operation");
  }
  process.stdout.write(JSON.stringify({ result }) + "\n");
});
'''
        self.process = subprocess.Popen(
            ["node", "-e", source],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            encoding="utf-8",
        )

    def call(self, op, value):
        if self.process.stdin is None or self.process.stdout is None:
            raise MachineError("SLEY_MACHINE_TEXT_RUNTIME_FAILURE", "text runtime pipe is unavailable")
        self.process.stdin.write(canonical_json({"op": op, "value": value}) + "\n")
        self.process.stdin.flush()
        response = self.process.stdout.readline()
        if response == "":
            raise MachineError("SLEY_MACHINE_TEXT_RUNTIME_FAILURE", "text runtime terminated unexpectedly")
        return json.loads(response)["result"]

    def close(self):
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                self.process.kill()


NODE_TEXT = NodeTextRuntime()


def require_intrinsic_args(qname, args, count):
    if len(args) != count:
        raise MachineError(
            "SLEY_MACHINE_ARITY_MISMATCH",
            f"intrinsic `{qname}` requires {count} arguments",
            exit_code=70,
        )


def eval_intrinsic(qname, args):
    if qname in {
        "sley.machine.text.nfkc_trim",
        "sley.machine.text.ascii_fold_en",
        "sley.machine.text.utf16_length",
        "sley.machine.text.has_prohibited_controls",
    }:
        require_intrinsic_args(qname, args, 1)
        if not isinstance(args[0], str):
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", f"intrinsic `{qname}` requires Text", exit_code=70)
        return NODE_TEXT.call(qname.rsplit(".", 1)[1], args[0])
    if qname == "sley.machine.text.characters":
        require_intrinsic_args(qname, args, 1)
        if not isinstance(args[0], str):
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "characters requires Text", exit_code=70)
        return list(args[0])
    if qname == "sley.machine.text.slice":
        require_intrinsic_args(qname, args, 3)
        if not isinstance(args[0], str) or any(not isinstance(value, int) or isinstance(value, bool) for value in args[1:]):
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "slice requires Text, Int, Int", exit_code=70)
        return args[0][args[1]:args[2]]
    if qname == "sley.machine.text.to_int":
        require_intrinsic_args(qname, args, 1)
        if not isinstance(args[0], str) or re.fullmatch(r"[0-9]+", args[0]) is None:
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "to_int requires non-empty ASCII digits", exit_code=70)
        return int(args[0])
    if qname == "sley.machine.int.to_text":
        require_intrinsic_args(qname, args, 1)
        if not isinstance(args[0], int) or isinstance(args[0], bool):
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "to_text requires Int", exit_code=70)
        return str(args[0])
    if qname == "sley.machine.int.digits":
        require_intrinsic_args(qname, args, 1)
        if not isinstance(args[0], int) or isinstance(args[0], bool) or args[0] < 0:
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "digits requires a non-negative Int", exit_code=70)
        return [int(character) for character in str(args[0])]
    if qname == "sley.machine.int.modulo":
        require_intrinsic_args(qname, args, 2)
        if any(not isinstance(value, int) or isinstance(value, bool) for value in args) or args[0] < 0 or args[1] <= 0:
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "modulo requires non-negative Int and positive Int", exit_code=70)
        return args[0] % args[1]
    if qname == "sley.machine.map.has_key":
        require_intrinsic_args(qname, args, 2)
        if not isinstance(args[0], dict) or not isinstance(args[1], str):
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "has_key requires Map and Text", exit_code=70)
        return args[1] in args[0]
    if qname == "sley.machine.map.keys":
        require_intrinsic_args(qname, args, 1)
        if not isinstance(args[0], dict):
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "keys requires Map", exit_code=70)
        return sorted(args[0])
    if qname == "sley.machine.json.kind":
        require_intrinsic_args(qname, args, 1)
        value = args[0]
        if value is None:
            return "Null"
        if isinstance(value, bool):
            return "Bool"
        if isinstance(value, int):
            return "Int"
        if isinstance(value, str):
            return "Text"
        if isinstance(value, list):
            return "List"
        if isinstance(value, dict):
            return "Map"
        return "Unknown"
    if qname == "sley.machine.date.is_gregorian":
        require_intrinsic_args(qname, args, 1)
        if not isinstance(args[0], str):
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "is_gregorian requires Text", exit_code=70)
        match = re.fullmatch(r"([0-9]{4})-([0-9]{2})-([0-9]{2})", args[0])
        if match is None:
            return False
        year, month, day = (int(value) for value in match.groups())
        if month < 1 or month > 12:
            return False
        leap = year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)
        month_lengths = [31, 29 if leap else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        return 1 <= day <= month_lengths[month - 1]
    return None


INTRINSIC_TASKS = {
    "sley.machine.text.nfkc_trim",
    "sley.machine.text.ascii_fold_en",
    "sley.machine.text.utf16_length",
    "sley.machine.text.has_prohibited_controls",
    "sley.machine.text.characters",
    "sley.machine.text.slice",
    "sley.machine.text.to_int",
    "sley.machine.int.to_text",
    "sley.machine.int.digits",
    "sley.machine.int.modulo",
    "sley.machine.map.has_key",
    "sley.machine.map.keys",
    "sley.machine.json.kind",
    "sley.machine.date.is_gregorian",
}


class Task:
    def __init__(self, module, name, return_type, effects, takes, body):
        self.module = module
        self.name = name
        self.return_type = return_type
        self.effects = effects
        self.takes = takes
        self.body = body

    @property
    def qname(self):
        return f"{self.module}.{self.name}"


def strip_comment(line):
    in_string = False
    escaped = False
    out = []
    for index, char in enumerate(line):
        if escaped:
            out.append(char)
            escaped = False
            continue
        if char == "\\" and in_string:
            out.append(char)
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            out.append(char)
            continue
        if not in_string and char == "/" and index + 1 < len(line) and line[index + 1] == "/":
            break
        out.append(char)
    return "".join(out)


def split_effects(text):
    if not text:
        return []
    return [item.strip() for item in re.split(r"[, ]+", text) if item.strip()]


def parse_source_file(path):
    module = "main"
    imports = {}
    tasks = []
    lines = path.read_text(encoding="utf-8").splitlines()
    index = 0
    while index < len(lines):
        raw = strip_comment(lines[index]).rstrip()
        stripped = raw.strip()
        module_match = re.match(r"^module\s+([A-Za-z_][A-Za-z0-9_.]*)\s*$", stripped)
        if module_match:
            module = module_match.group(1)
            index += 1
            continue
        import_match = re.match(r"^import\s+([A-Za-z_][A-Za-z0-9_.]*)(?:\s+as\s+([A-Za-z_][A-Za-z0-9_]*))?\s*$", stripped)
        if import_match:
            imported = import_match.group(1)
            alias = import_match.group(2) or imported.split(".")[-1]
            imports[alias] = imported
            index += 1
            continue
        task_match = re.match(r"^(?:export\s+)?task\s+([A-Za-z_][A-Za-z0-9_]*)\s*->\s*(.*?)\s*\{\s*$", stripped)
        if not task_match:
            index += 1
            continue
        name = task_match.group(1)
        tail = task_match.group(2).strip()
        if " uses " in f" {tail} ":
            return_type, effects_text = re.split(r"\s+uses\s+", tail, maxsplit=1)
        else:
            return_type, effects_text = tail, ""
        body = []
        takes = []
        depth = raw.count("{") - raw.count("}")
        index += 1
        while index < len(lines) and depth > 0:
            line = strip_comment(lines[index]).rstrip()
            delta = line.count("{") - line.count("}")
            if depth + delta <= 0 and line.strip() == "}":
                depth += delta
                index += 1
                break
            body.append(line)
            take_match = re.match(r"^\s*take(?:\s+gate)?\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.*?)\s*$", line.strip())
            if take_match and not line.strip().startswith("take gate "):
                takes.append({"name": take_match.group(1), "type": take_match.group(2).strip()})
            depth += delta
            index += 1
        tasks.append(Task(module, name, return_type.strip(), split_effects(effects_text), takes, body))
    return imports, tasks


IMPORTS = {}
TASKS = {}
TASKS_BY_NAME = {}
for source_file in SOURCE_FILES:
    imports, tasks = parse_source_file(source_file)
    for task in tasks:
        TASKS[task.qname] = task
        TASKS_BY_NAME.setdefault(task.name, []).append(task.qname)
        IMPORTS.setdefault(task.module, {}).update(imports)


def error_response(request_id, error, counters=None):
    return {
        "schema": SCHEMA,
        "protocolVersion": PROTOCOL_VERSION,
        "requestId": request_id,
        "status": "error",
        "package": PACKAGE_INFO,
        "source": SOURCE_INFO,
        "runtime": RUNTIME_INFO,
        "error": error,
        "counters": counters or empty_counters(),
    }


def empty_counters():
    return {
        "steps": 0,
        "calls": 0,
        "maxCallDepth": 0,
        "inputBytes": 0,
        "outputBytes": 0,
        "batchIndex": 0,
    }


def emit(value, response_limit=None):
    counters = value.get("counters")
    if isinstance(counters, dict):
        counters["outputBytes"] = 0
        for _ in range(8):
            line = canonical_json(value)
            output_bytes = len(line.encode("utf-8")) + 1
            if counters.get("outputBytes") == output_bytes:
                break
            counters["outputBytes"] = output_bytes
    line = canonical_json(value)
    output_bytes = len(line.encode("utf-8")) + 1
    if response_limit is not None and output_bytes > response_limit:
        raise MachineError(
            "SLEY_MACHINE_RESPONSE_LIMIT_EXCEEDED",
            "canonical response exceeded responseBytes limit",
            exit_code=71,
            failure_class="resource",
            phase="output",
            path="/limits/responseBytes",
            limit="responseBytes",
        )
    sys.stdout.write(line + "\n")
    sys.stdout.flush()


class Limits:
    def __init__(self, merged, input_bytes, batch_index):
        self.values = merged
        self.steps = 0
        self.calls = 0
        self.max_depth = 0
        self.input_bytes = input_bytes
        self.batch_index = batch_index

    def step(self):
        self.steps += 1
        if self.steps > self.values["evaluationSteps"]:
            raise MachineError(
                "SLEY_MACHINE_STEP_LIMIT_EXCEEDED",
                "evaluation exceeded evaluationSteps limit",
                exit_code=71,
                failure_class="resource",
                phase="execution",
                path="/limits/evaluationSteps",
                limit="evaluationSteps",
                counters=self.counters(),
            )

    def enter_call(self, depth):
        self.calls += 1
        self.max_depth = max(self.max_depth, depth)
        if depth > self.values["callDepth"]:
            raise MachineError(
                "SLEY_MACHINE_CALL_DEPTH_EXCEEDED",
                "evaluation exceeded callDepth limit",
                exit_code=71,
                failure_class="resource",
                phase="execution",
                path="/limits/callDepth",
                limit="callDepth",
                counters=self.counters(),
            )

    def check_collection(self, value, path="/result"):
        if isinstance(value, (list, tuple)):
            if len(value) > self.values["collectionLength"]:
                raise MachineError(
                    "SLEY_MACHINE_COLLECTION_LIMIT_EXCEEDED",
                    "collection exceeded collectionLength limit",
                    exit_code=71,
                    failure_class="resource",
                    phase="execution",
                    path=path,
                    limit="collectionLength",
                    counters=self.counters(),
                )
            for index, item in enumerate(value):
                self.check_collection(item, f"{path}/{index}")
        elif isinstance(value, dict):
            if len(value) > self.values["collectionLength"]:
                raise MachineError(
                    "SLEY_MACHINE_COLLECTION_LIMIT_EXCEEDED",
                    "collection exceeded collectionLength limit",
                    exit_code=71,
                    failure_class="resource",
                    phase="execution",
                    path=path,
                    limit="collectionLength",
                    counters=self.counters(),
                )
            for key, item in value.items():
                self.check_collection(item, f"{path}/{key}")

    def counters(self):
        return {
            "steps": self.steps,
            "calls": self.calls,
            "maxCallDepth": self.max_depth,
            "inputBytes": self.input_bytes,
            "outputBytes": 0,
            "batchIndex": self.batch_index,
        }


def normalize_limits(raw):
    if raw is None:
        return dict(DEFAULT_LIMITS)
    if not isinstance(raw, dict):
        raise MachineError(
            "SLEY_MACHINE_REQUEST_SCHEMA",
            "limits must be an object",
            exit_code=65,
            failure_class="schema",
            phase="input",
            path="/limits",
        )
    unknown = sorted(set(raw) - LIMIT_FIELDS)
    if unknown:
        raise MachineError(
            "SLEY_MACHINE_REQUEST_SCHEMA",
            f"unknown limits field `{unknown[0]}`",
            exit_code=65,
            failure_class="schema",
            phase="input",
            path=f"/limits/{unknown[0]}",
        )
    limits = dict(DEFAULT_LIMITS)
    for key, value in raw.items():
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            raise MachineError(
                "SLEY_MACHINE_REQUEST_SCHEMA",
                f"limits.{key} must be a positive integer",
                exit_code=65,
                failure_class="schema",
                phase="input",
                path=f"/limits/{key}",
            )
        if value > DEFAULT_LIMITS[key]:
            raise MachineError(
                "SLEY_MACHINE_REQUEST_SCHEMA",
                f"limits.{key} cannot exceed the runtime default",
                exit_code=65,
                failure_class="schema",
                phase="input",
                path=f"/limits/{key}",
            )
        limits[key] = value
    return limits


def validate_request(request):
    if not isinstance(request, dict):
        raise MachineError(
            "SLEY_MACHINE_REQUEST_SCHEMA",
            "request must be a JSON object",
            exit_code=65,
            failure_class="schema",
            phase="input",
            path="/",
        )
    unknown = sorted(set(request) - REQUEST_FIELDS)
    if unknown:
        raise MachineError(
            "SLEY_MACHINE_REQUEST_SCHEMA",
            f"unknown request field `{unknown[0]}`",
            exit_code=65,
            failure_class="schema",
            phase="input",
            path=f"/{unknown[0]}",
        )
    for key in ("protocolVersion", "requestId", "operation", "packageId", "packageVersion", "task", "input"):
        if key not in request:
            raise MachineError(
                "SLEY_MACHINE_REQUEST_SCHEMA",
                f"missing required request field `{key}`",
                exit_code=65,
                failure_class="schema",
                phase="input",
                path=f"/{key}",
            )
    if request["protocolVersion"] != PROTOCOL_VERSION:
        raise MachineError(
            "SLEY_MACHINE_PROTOCOL_MISMATCH",
            "request protocolVersion does not match this machine",
            exit_code=64,
            failure_class="framing",
            phase="input",
            path="/protocolVersion",
        )
    if not isinstance(request["requestId"], str) or request["requestId"] == "":
        raise MachineError(
            "SLEY_MACHINE_REQUEST_SCHEMA",
            "requestId must be a non-empty string",
            exit_code=65,
            failure_class="schema",
            phase="input",
            path="/requestId",
        )
    if request["operation"] != "invoke":
        raise MachineError(
            "SLEY_MACHINE_REQUEST_SCHEMA",
            "operation must be `invoke`",
            exit_code=65,
            failure_class="schema",
            phase="input",
            path="/operation",
        )
    if request["packageId"] != PACKAGE_ID:
        raise MachineError(
            "SLEY_MACHINE_PIN_MISMATCH",
            "request packageId does not match the declared package",
            exit_code=72,
            failure_class="pin",
            phase="input",
            path="/packageId",
        )
    if request["packageVersion"] != PACKAGE_VERSION:
        raise MachineError(
            "SLEY_MACHINE_PIN_MISMATCH",
            "request packageVersion does not match the declared package",
            exit_code=72,
            failure_class="pin",
            phase="input",
            path="/packageVersion",
        )
    if not isinstance(request["task"], str) or "." not in request["task"]:
        raise MachineError(
            "SLEY_MACHINE_REQUEST_SCHEMA",
            "task must be a qualified task name",
            exit_code=65,
            failure_class="schema",
            phase="input",
            path="/task",
        )
    if not isinstance(request["input"], dict):
        raise MachineError(
            "SLEY_MACHINE_REQUEST_SCHEMA",
            "input must be a JSON object",
            exit_code=65,
            failure_class="schema",
            phase="input",
            path="/input",
        )
    pins = request.get("pins") or {}
    if not isinstance(pins, dict):
        raise MachineError(
            "SLEY_MACHINE_REQUEST_SCHEMA",
            "pins must be an object",
            exit_code=65,
            failure_class="schema",
            phase="input",
            path="/pins",
        )
    unknown_pins = sorted(set(pins) - PIN_FIELDS)
    if unknown_pins:
        raise MachineError(
            "SLEY_MACHINE_REQUEST_SCHEMA",
            f"unknown pins field `{unknown_pins[0]}`",
            exit_code=65,
            failure_class="schema",
            phase="input",
            path=f"/pins/{unknown_pins[0]}",
        )
    expected_pins = {
        "sourceDigest": SOURCE_DIGEST,
        "runtimeId": RUNTIME_ID,
        "runtimeVersion": RUNTIME_VERSION,
        "runtimeDigest": RUNTIME_DIGEST,
    }
    for key, expected in expected_pins.items():
        if key in pins and pins[key] != expected:
            raise MachineError(
                "SLEY_MACHINE_PIN_MISMATCH",
                f"request pin {key} does not match this machine",
                exit_code=72,
                failure_class="pin",
                phase="input",
                path=f"/pins/{key}",
            )


def strip_outer_parens(expr):
    expr = expr.strip()
    if not (expr.startswith("(") and expr.endswith(")")):
        return expr
    depth = 0
    in_string = False
    escaped = False
    for index, char in enumerate(expr):
        if escaped:
            escaped = False
            continue
        if char == "\\" and in_string:
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if in_string:
            continue
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0 and index != len(expr) - 1:
                return expr
    return strip_outer_parens(expr[1:-1]) if depth == 0 else expr


def split_top_level(text, token):
    depth = 0
    in_string = False
    escaped = False
    index = 0
    while index <= len(text) - len(token):
        char = text[index]
        if escaped:
            escaped = False
            index += 1
            continue
        if char == "\\" and in_string:
            escaped = True
            index += 1
            continue
        if char == '"':
            in_string = not in_string
            index += 1
            continue
        if not in_string:
            if char in "([{":
                depth += 1
            elif char in ")]}":
                depth -= 1
            elif depth == 0 and text.startswith(token, index):
                left = text[:index].strip()
                right = text[index + len(token):].strip()
                if left and right:
                    return left, right
        index += 1
    return None


def split_comparison(text):
    for token in (">=", "<=", "==", "!=", ">", "<"):
        split = split_top_level(text, token)
        if split:
            return split[0], token, split[1]
    return None


def split_commas(text):
    items = []
    depth = 0
    in_string = False
    escaped = False
    start = 0
    for index, char in enumerate(text):
        if escaped:
            escaped = False
            continue
        if char == "\\" and in_string:
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if in_string:
            continue
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        elif char == "," and depth == 0:
            item = text[start:index].strip()
            if item:
                items.append(item)
            start = index + 1
    tail = text[start:].strip()
    if tail:
        items.append(tail)
    return items


def split_colon(text):
    depth = 0
    in_string = False
    escaped = False
    for index, char in enumerate(text):
        if escaped:
            escaped = False
            continue
        if char == "\\" and in_string:
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if in_string:
            continue
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        elif char == ":" and depth == 0:
            return text[:index].strip(), text[index + 1:].strip()
    return None


def trailing_postfix(text, opener, closer):
    text = text.strip()
    if not text.endswith(closer):
        return None
    depth = 0
    in_string = False
    escaped = False
    for index in range(len(text) - 1, -1, -1):
        char = text[index]
        if escaped:
            escaped = False
            continue
        if char == "\\" and in_string:
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if in_string:
            continue
        if char == closer:
            depth += 1
        elif char == opener:
            depth -= 1
            if depth == 0:
                base = text[:index].strip()
                inner = text[index + 1:-1].strip()
                if base:
                    return base, inner
                return None
    return None


def trailing_field(text):
    depth = 0
    in_string = False
    escaped = False
    for index in range(len(text) - 1, -1, -1):
        char = text[index]
        if escaped:
            escaped = False
            continue
        if char == "\\" and in_string:
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if in_string:
            continue
        if char in ")]}":
            depth += 1
        elif char in "([{":
            depth -= 1
        elif char == "." and depth == 0:
            base = text[:index].strip()
            field = text[index + 1:].strip()
            if base and re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", field):
                return base, field
    return None


def balance(text):
    depth = 0
    in_string = False
    escaped = False
    for char in text:
        if escaped:
            escaped = False
            continue
        if char == "\\" and in_string:
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if in_string:
            continue
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
    return depth


def coerce_bool(value):
    if isinstance(value, bool):
        return value
    raise MachineError(
        "SLEY_MACHINE_TYPE_ERROR",
        "condition did not evaluate to Bool",
        exit_code=70,
        failure_class="runtime",
        phase="execution",
    )


class Evaluator:
    def __init__(self, limits):
        self.limits = limits

    def resolve_task(self, current_module, callee):
        if "." in callee:
            prefix, rest = callee.split(".", 1)
            imported = IMPORTS.get(current_module, {}).get(prefix)
            qname = f"{imported}.{rest}" if imported else callee
            if qname in TASKS:
                return qname
            raise MachineError(
                "SLEY_MACHINE_TASK_NOT_FOUND",
                f"task `{callee}` was not found",
                exit_code=70,
                failure_class="runtime",
                phase="execution",
                path="/task",
            )
        qname = f"{current_module}.{callee}"
        if qname in TASKS:
            return qname
        candidates = TASKS_BY_NAME.get(callee, [])
        if len(candidates) == 1:
            return candidates[0]
        raise MachineError(
            "SLEY_MACHINE_TASK_NOT_FOUND",
            f"task `{callee}` was not found",
            exit_code=70,
            failure_class="runtime",
            phase="execution",
            path="/task",
        )

    def call_task(self, qname, args, depth):
        self.limits.enter_call(depth)
        task = TASKS.get(qname)
        if task is None:
            raise MachineError(
                "SLEY_MACHINE_TASK_NOT_FOUND",
                f"task `{qname}` was not found",
                exit_code=70,
                failure_class="runtime",
                phase="input",
                path="/task",
            )
        if task.effects:
            raise MachineError(
                "SLEY_MACHINE_EFFECTFUL_TASK",
                "machine invocation accepts only pure Sley tasks",
                exit_code=70,
                failure_class="runtime",
                phase="execution",
                path="/task",
            )
        if len(args) != len(task.takes):
            raise MachineError(
                "SLEY_MACHINE_ARITY_MISMATCH",
                "structured input did not match task takes",
                exit_code=65,
                failure_class="schema",
                phase="input",
                path="/input",
            )
        env = {take["name"]: args[index] for index, take in enumerate(task.takes)}
        try:
            self.eval_block(task.body, env, task, depth)
        except ReturnValue as returned:
            self.limits.check_collection(returned.value)
            return returned.value
        if task.return_type != "Unit":
            raise MachineError(
                "SLEY_MACHINE_MISSING_RETURN",
                "task completed without a return value",
                exit_code=70,
                failure_class="runtime",
                phase="execution",
                path="/task",
            )
        return None

    def prepare_top_level_args(self, task, input_value):
        self.limits.check_collection(input_value, "/input")
        if not task.takes:
            if input_value:
                raise MachineError(
                    "SLEY_MACHINE_REQUEST_SCHEMA",
                    "zero-take task requires an empty input object",
                    exit_code=65,
                    failure_class="schema",
                    phase="input",
                    path="/input",
                )
            return []
        if len(task.takes) == 1 and task.takes[0]["name"] == "input":
            return [input_value]
        take_names = [take["name"] for take in task.takes]
        missing = [name for name in take_names if name not in input_value]
        unknown = sorted(set(input_value) - set(take_names))
        if missing or unknown:
            path = f"/input/{missing[0]}" if missing else f"/input/{unknown[0]}"
            message = "input is missing required task take" if missing else "input contains an unknown task take"
            raise MachineError(
                "SLEY_MACHINE_REQUEST_SCHEMA",
                message,
                exit_code=65,
                failure_class="schema",
                phase="input",
                path=path,
            )
        return [input_value[name] for name in take_names]

    def eval_block(self, lines, env, task, depth):
        index = 0
        while index < len(lines):
            raw = lines[index]
            line = raw.strip()
            if not line or line == "}" or line.startswith("take "):
                index += 1
                continue
            self.limits.step()
            if line.startswith("if ") and line.endswith("{"):
                condition = line[3:-1].strip()
                then_lines, else_lines, index = self.collect_if(lines, index)
                branch = then_lines if coerce_bool(self.eval_expr(condition, env, task, depth)) else else_lines
                self.eval_block(branch, env, task, depth)
                continue
            if line.startswith("while ") and line.endswith("{"):
                condition = line[6:-1].strip()
                body, index = self.collect_block(lines, index)
                while coerce_bool(self.eval_expr(condition, env, task, depth)):
                    self.eval_block(body, env, task, depth)
                continue
            each_match = re.match(r"^each\s+([A-Za-z_][A-Za-z0-9_]*)\s+in\s+(.+)\s*\{\s*$", line)
            if each_match:
                item_name = each_match.group(1)
                collection_expr = each_match.group(2)
                body, index = self.collect_block(lines, index)
                collection = self.eval_expr(collection_expr, env, task, depth)
                if not isinstance(collection, list):
                    raise MachineError("SLEY_MACHINE_TYPE_ERROR", "each requires a List value", exit_code=70)
                for item in collection:
                    env[item_name] = item
                    self.eval_block(body, env, task, depth)
                continue
            if line.startswith("return "):
                expr, index = self.collect_expression(lines, index, line[len("return "):].strip())
                raise ReturnValue(self.eval_expr(expr, env, task, depth))
            assign_match = re.match(r"^(bind|state|tally|set)\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*:[^=]+)?\s*=\s*(.+)$", line)
            if assign_match:
                name = assign_match.group(2)
                expr, index = self.collect_expression(lines, index, assign_match.group(3).strip())
                if expr.endswith("?"):
                    value = self.eval_expr(expr[:-1], env, task, depth)
                    if isinstance(value, dict) and set(value) == {"ok"}:
                        value = value["ok"]
                    elif isinstance(value, dict) and set(value) == {"err"}:
                        raise ReturnValue(value)
                else:
                    value = self.eval_expr(expr, env, task, depth)
                self.limits.check_collection(value, f"/local/{name}")
                env[name] = value
                continue
            if line.startswith("call "):
                self.eval_expr(line, env, task, depth)
                index += 1
                continue
            raise MachineError(
                "SLEY_MACHINE_UNSUPPORTED_STATEMENT",
                f"statement is outside the machine evaluator subset: `{line}`",
                exit_code=70,
                failure_class="runtime",
                phase="execution",
                path="/task/body",
            )

    def collect_expression(self, lines, index, expr):
        while balance(expr) > 0 and index + 1 < len(lines):
            index += 1
            expr += "\n" + lines[index].strip()
        return expr.strip(), index + 1

    def collect_block(self, lines, start):
        header = lines[start]
        depth = header.count("{") - header.count("}")
        body = []
        index = start + 1
        while index < len(lines):
            line = lines[index]
            stripped = line.strip()
            delta = line.count("{") - line.count("}")
            if depth + delta <= 0 and stripped == "}":
                return body, index + 1
            body.append(line)
            depth += delta
            index += 1
        raise MachineError("SLEY_MACHINE_PARSE_ERROR", "unterminated block", exit_code=70)

    def collect_if(self, lines, start):
        header = lines[start]
        depth = header.count("{") - header.count("}")
        then_lines = []
        else_lines = []
        index = start + 1
        saw_else = False
        while index < len(lines):
            line = lines[index]
            stripped = line.strip()
            delta = line.count("{") - line.count("}")
            if depth == 1 and stripped.startswith("} else"):
                saw_else = True
                depth = 1
                index += 1
                continue
            if depth + delta <= 0 and stripped == "}":
                return then_lines, else_lines, index + 1
            if saw_else:
                else_lines.append(line)
            else:
                then_lines.append(line)
            depth += delta
            index += 1
        raise MachineError("SLEY_MACHINE_PARSE_ERROR", "unterminated if block", exit_code=70)

    def eval_expr(self, expr, env, task, depth):
        self.limits.step()
        expr = strip_outer_parens(expr.strip())
        if not expr:
            raise MachineError("SLEY_MACHINE_EXPRESSION_ERROR", "empty expression", exit_code=70)
        if expr.endswith("?"):
            value = self.eval_expr(expr[:-1], env, task, depth)
            if isinstance(value, dict) and set(value) == {"ok"}:
                return value["ok"]
            if isinstance(value, dict) and set(value) == {"err"}:
                raise ReturnValue(value)
            return value
        inline_if = re.match(r"^if\s+(.+?)\s*\{\s*(.+)\s*\}\s*else\s*\{\s*(.+)\s*\}$", expr, flags=re.S)
        if inline_if:
            condition = self.eval_expr(inline_if.group(1), env, task, depth)
            return self.eval_expr(inline_if.group(2) if coerce_bool(condition) else inline_if.group(3), env, task, depth)
        for token in ("||", "&&"):
            split = split_top_level(expr, token)
            if split:
                left = coerce_bool(self.eval_expr(split[0], env, task, depth))
                right = coerce_bool(self.eval_expr(split[1], env, task, depth))
                return (left or right) if token == "||" else (left and right)
        comparison = split_comparison(expr)
        if comparison:
            left = self.eval_expr(comparison[0], env, task, depth)
            right = self.eval_expr(comparison[2], env, task, depth)
            op = comparison[1]
            if op == "==":
                return left == right
            if op == "!=":
                return left != right
            if op == ">":
                return left > right
            if op == "<":
                return left < right
            if op == ">=":
                return left >= right
            if op == "<=":
                return left <= right
        split = split_top_level(expr, "+")
        if split:
            left = self.eval_expr(split[0], env, task, depth)
            right = self.eval_expr(split[1], env, task, depth)
            if isinstance(left, int) and not isinstance(left, bool) and isinstance(right, int) and not isinstance(right, bool):
                return left + right
            return f"{left}{right}"
        split = split_top_level(expr, "*")
        if split:
            left = self.eval_expr(split[0], env, task, depth)
            right = self.eval_expr(split[1], env, task, depth)
            if isinstance(left, int) and not isinstance(left, bool) and isinstance(right, int) and not isinstance(right, bool):
                return left * right
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "multiplication requires Int values", exit_code=70)
        if expr.startswith("!"):
            return not coerce_bool(self.eval_expr(expr[1:], env, task, depth))
        field = trailing_field(expr)
        if field:
            base = self.eval_expr(field[0], env, task, depth)
            if isinstance(base, dict) and field[1] in base:
                return base[field[1]]
            raise MachineError("SLEY_MACHINE_FIELD_ERROR", "record field was not found", exit_code=70)
        indexed = trailing_postfix(expr, "[", "]")
        if indexed:
            base = self.eval_expr(indexed[0], env, task, depth)
            key = self.eval_expr(indexed[1], env, task, depth)
            if isinstance(base, list) and isinstance(key, int) and not isinstance(key, bool) and 0 <= key < len(base):
                return base[key]
            if isinstance(base, dict):
                key_text = str(key)
                if key_text in base:
                    return base[key_text]
            raise MachineError("SLEY_MACHINE_INDEX_ERROR", "collection index was not found", exit_code=70)
        if expr.startswith("call "):
            return self.eval_call(expr[len("call "):].strip(), env, task, depth)
        call_match = re.match(r"^([A-Za-z_][A-Za-z0-9_.]*)\((.*)\)$", expr, flags=re.S)
        if call_match and call_match.group(1) == "len":
            value = self.eval_expr(call_match.group(2), env, task, depth)
            if isinstance(value, (list, dict, str)):
                return len(value)
            raise MachineError("SLEY_MACHINE_TYPE_ERROR", "len requires Text, List, or Map", exit_code=70)
        if expr.startswith("Ok(") and expr.endswith(")"):
            return {"ok": self.eval_expr(expr[3:-1], env, task, depth)}
        if expr.startswith("Err(") and expr.endswith(")"):
            return {"err": self.eval_expr(expr[4:-1], env, task, depth)}
        if expr.startswith("[") and expr.endswith("]"):
            value = [self.eval_expr(item, env, task, depth) for item in split_commas(expr[1:-1])]
            self.limits.check_collection(value)
            return value
        map_match = re.match(r"^map\s*\{(.*)\}$", expr, flags=re.S)
        if map_match:
            result = {}
            for item in split_commas(map_match.group(1)):
                split = split_colon(item)
                if split is None:
                    continue
                raw_key, raw_value = split
                if raw_key.startswith('"') and raw_key.endswith('"'):
                    key = json.loads(raw_key)
                else:
                    key = str(self.eval_expr(raw_key, env, task, depth))
                result[str(key)] = self.eval_expr(raw_value, env, task, depth)
            self.limits.check_collection(result)
            return result
        record_match = re.match(r"^[A-Za-z_][A-Za-z0-9_]*\s*\{(.*)\}$", expr, flags=re.S)
        if record_match:
            result = {}
            for item in split_commas(record_match.group(1)):
                split = split_colon(item)
                if split is None:
                    continue
                raw_key, raw_value = split
                key = raw_key.strip().strip('"')
                result[key] = self.eval_expr(raw_value, env, task, depth)
            self.limits.check_collection(result)
            return result
        if expr == "true":
            return True
        if expr == "false":
            return False
        if re.match(r"^-?[0-9]+$", expr):
            return int(expr)
        if expr.startswith('"') and expr.endswith('"'):
            return json.loads(expr)
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", expr):
            if expr in env:
                return env[expr]
            raise MachineError("SLEY_MACHINE_IDENTIFIER_NOT_FOUND", "identifier was not found", exit_code=70)
        raise MachineError(
            "SLEY_MACHINE_UNSUPPORTED_EXPRESSION",
            "expression is outside the machine evaluator subset",
            exit_code=70,
            failure_class="runtime",
            phase="execution",
            path="/task/body",
        )

    def eval_call(self, call_source, env, task, depth):
        call_source = call_source.rstrip("?").strip()
        match = re.match(r"^([A-Za-z_][A-Za-z0-9_.]*)\((.*)\)$", call_source, flags=re.S)
        if not match:
            raise MachineError("SLEY_MACHINE_CALL_PARSE_ERROR", "call expression could not be parsed", exit_code=70)
        callee = match.group(1)
        args = [self.eval_expr(item, env, task, depth) for item in split_commas(match.group(2))]
        qname = self.resolve_task(task.module, callee)
        if qname in INTRINSIC_TASKS:
            self.limits.enter_call(depth + 1)
            value = eval_intrinsic(qname, args)
            self.limits.check_collection(value)
            return value
        return self.call_task(qname, args, depth + 1)


def request_id_from(value):
    if isinstance(value, dict) and isinstance(value.get("requestId"), str):
        return value["requestId"]
    return None


def error_from_exception(exc):
    error = {
        "class": exc.failure_class,
        "code": exc.code,
        "message": exc.message,
        "phase": exc.phase,
        "path": exc.path,
    }
    if exc.limit:
        error["limit"] = exc.limit
    return error


def timeout_handler(_signum, _frame):
    raise MachineError(
        "SLEY_MACHINE_TIMEOUT",
        "evaluation exceeded timeoutMs limit",
        exit_code=71,
        failure_class="resource",
        phase="execution",
        path="/limits/timeoutMs",
        limit="timeoutMs",
    )


def process_request(request, input_bytes, batch_index):
    validate_request(request)
    if not CHECK_PASSED:
        raise MachineError(
            "SLEY_MACHINE_CHECK_FAILURE",
            "Sley source did not pass the checker",
            exit_code=70,
            failure_class="check",
            phase="check",
            path="/source",
        )
    limits_map = normalize_limits(request.get("limits"))
    if input_bytes > limits_map["requestLineBytes"]:
        raise MachineError(
            "SLEY_MACHINE_LINE_LIMIT_EXCEEDED",
            "request line exceeded requestLineBytes limit",
            exit_code=64,
            failure_class="framing",
            phase="input",
            path="/limits/requestLineBytes",
            limit="requestLineBytes",
        )
    if batch_index > limits_map["batch"]:
        raise MachineError(
            "SLEY_MACHINE_BATCH_LIMIT_EXCEEDED",
            "batch exceeded batch limit",
            exit_code=71,
            failure_class="resource",
            phase="input",
            path="/limits/batch",
            limit="batch",
        )
    qname = request["task"]
    task = TASKS.get(qname)
    if task is None:
        raise MachineError(
            "SLEY_MACHINE_TASK_NOT_FOUND",
            "requested task was not found",
            exit_code=70,
            failure_class="runtime",
            phase="input",
            path="/task",
        )
    limits = Limits(limits_map, input_bytes, batch_index)
    evaluator = Evaluator(limits)
    args = evaluator.prepare_top_level_args(task, request["input"])
    signal.signal(signal.SIGALRM, timeout_handler)
    signal.setitimer(signal.ITIMER_REAL, limits_map["timeoutMs"] / 1000.0)
    try:
        result = evaluator.call_task(qname, args, 1)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    normalized_input = json.loads(canonical_json(request["input"]))
    payload = {
        "package": PACKAGE_INFO,
        "runtime": RUNTIME_INFO,
        "source": SOURCE_INFO,
        "task": qname,
        "input": normalized_input,
        "result": result,
    }
    return {
        "schema": SCHEMA,
        "protocolVersion": PROTOCOL_VERSION,
        "requestId": request["requestId"],
        "status": "ok",
        "package": PACKAGE_INFO,
        "source": SOURCE_INFO,
        "runtime": RUNTIME_INFO,
        "task": qname,
        "input": normalized_input,
        "result": result,
        "digests": {
            "input": digest_json(normalized_input),
            "result": digest_json(result),
            "payload": digest_json(payload),
        },
        "counters": limits.counters(),
    }, limits_map["responseBytes"]


def main():
    exit_code = 0
    batch_index = 0
    for raw_line in sys.stdin.buffer:
        if raw_line in (b"\n", b"\r\n"):
            continue
        batch_index += 1
        input_bytes = len(raw_line)
        request = None
        try:
            if input_bytes > DEFAULT_LIMITS["requestLineBytes"]:
                raise MachineError(
                    "SLEY_MACHINE_LINE_LIMIT_EXCEEDED",
                    "request line exceeded requestLineBytes limit",
                    exit_code=64,
                    failure_class="framing",
                    phase="input",
                    path="/limits/requestLineBytes",
                    limit="requestLineBytes",
                )
            try:
                request = json.loads(raw_line.decode("utf-8"))
            except UnicodeDecodeError as exc:
                raise MachineError(
                    "SLEY_MACHINE_UTF8_ERROR",
                    "request line was not valid UTF-8",
                    exit_code=64,
                    failure_class="framing",
                    phase="input",
                    path="/",
                ) from exc
            except json.JSONDecodeError as exc:
                raise MachineError(
                    "SLEY_MACHINE_JSON_PARSE_ERROR",
                    "request line was not valid JSON",
                    exit_code=64,
                    failure_class="framing",
                    phase="input",
                    path="/",
                ) from exc
            response, response_limit = process_request(request, input_bytes, batch_index)
            emit(response, response_limit)
        except MachineError as exc:
            counters = exc.counters or empty_counters()
            counters["inputBytes"] = input_bytes
            counters["batchIndex"] = batch_index
            response = error_response(request_id_from(request), error_from_exception(exc), counters)
            try:
                emit(response)
            except BrokenPipeError:
                return 74
            return exc.exit_code
        except BrokenPipeError:
            return 74
    NODE_TEXT.close()
    return exit_code


raise SystemExit(main())
PY
}

command_arena() {
  local arena_bin="$ROOT_DIR/bin/sley-arena"
  if [[ ! -x "$arena_bin" ]]; then
    echo "sley arena requires executable bin/sley-arena" >&2
    return 127
  fi
  exec "$arena_bin" "$@"
}
