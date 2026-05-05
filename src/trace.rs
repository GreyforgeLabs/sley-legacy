use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::ast::ProvenanceRecord;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceReceipt {
    pub schema: String,
    pub target: String,
    pub written_at: String,
    pub provenance: Vec<ProvenanceRecord>,
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
        schema: "sley.trace.receipt.v0".to_string(),
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

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}
