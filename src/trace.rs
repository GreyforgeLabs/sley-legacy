use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::ast::{Program, ProvenanceRecord};
use crate::symbols::build_symbol_graph;

pub const TRACE_RECEIPT_SCHEMA: &str = "sley.trace.receipt.v0";
pub const TRACE_SEAL_SCHEMA: &str = "sley.trace.seal.v0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceReceipt {
    pub schema: String,
    pub target: String,
    pub written_at: String,
    pub provenance: Vec<ProvenanceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceSeal {
    pub schema: String,
    pub target: String,
    pub source_digest: String,
    pub graph_digest: String,
    pub trace_digest: String,
    pub seal_digest: String,
    pub module_count: usize,
    pub task_count: usize,
    pub receipt_count: usize,
}

#[derive(Debug, Serialize)]
struct TraceSealMaterial<'a> {
    schema: &'a str,
    target: &'a str,
    source_digest: &'a str,
    graph_digest: &'a str,
    trace_digest: &'a str,
    module_count: usize,
    task_count: usize,
    receipt_count: usize,
}

pub fn default_trace_path(target: impl AsRef<Path>) -> PathBuf {
    let target = target.as_ref();
    let root = if target.is_dir() {
        target.to_path_buf()
    } else {
        target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    root.join(".sley").join("trace.jsonl")
}

pub fn build_trace_receipt(
    target: impl AsRef<Path>,
    provenance: Vec<ProvenanceRecord>,
) -> TraceReceipt {
    TraceReceipt {
        schema: TRACE_RECEIPT_SCHEMA.to_string(),
        target: target.as_ref().display().to_string(),
        written_at: now_rfc3339(),
        provenance,
    }
}

pub fn append_trace_receipt(path: impl AsRef<Path>, receipt: &TraceReceipt) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create trace directory {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open trace file {}", path.display()))?;
    serde_json::to_writer(&mut file, receipt)
        .with_context(|| format!("failed to serialize trace receipt {}", path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("failed to write trace receipt {}", path.display()))?;
    Ok(())
}

pub fn read_trace_receipts(path: impl AsRef<Path>) -> Result<Vec<TraceReceipt>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read trace file {}", path.display()))?;
    let mut receipts = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let receipt = serde_json::from_str::<TraceReceipt>(line).with_context(|| {
            format!(
                "failed to parse trace receipt {}:{}",
                path.display(),
                line_index + 1
            )
        })?;
        receipts.push(receipt);
    }
    Ok(receipts)
}

pub fn build_trace_seal(
    target: impl Into<String>,
    source_bytes: &[u8],
    program: &Program,
    receipts: &[TraceReceipt],
) -> Result<TraceSeal> {
    let target = target.into();
    let graph = build_symbol_graph(program);
    let graph_bytes = serde_json::to_vec(&graph).context("failed to serialize symbol graph")?;
    let trace_bytes = serde_json::to_vec(receipts).context("failed to serialize trace receipts")?;

    let source_digest = content_digest(source_bytes);
    let graph_digest = content_digest(&graph_bytes);
    let trace_digest = content_digest(&trace_bytes);
    let module_count = graph.modules.len();
    let task_count = graph
        .modules
        .iter()
        .map(|module| module.tasks.len())
        .sum::<usize>();
    let receipt_count = receipts.len();
    let material = TraceSealMaterial {
        schema: TRACE_SEAL_SCHEMA,
        target: &target,
        source_digest: &source_digest,
        graph_digest: &graph_digest,
        trace_digest: &trace_digest,
        module_count,
        task_count,
        receipt_count,
    };
    let material_bytes =
        serde_json::to_vec(&material).context("failed to serialize trace seal material")?;
    let seal_digest = content_digest(&material_bytes);

    Ok(TraceSeal {
        schema: TRACE_SEAL_SCHEMA.to_string(),
        target,
        source_digest,
        graph_digest,
        trace_digest,
        seal_digest,
        module_count,
        task_count,
        receipt_count,
    })
}

pub fn content_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("sha256:{}", hex_bytes(&digest))
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
