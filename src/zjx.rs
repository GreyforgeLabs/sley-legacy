use serde::Serialize;

use crate::symbols::{SymbolGraph, SymbolGraphSlice};
use crate::trace::{TraceReceipt, content_digest};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SleyZjxEnvelope {
    pub schema: String,
    pub format: String,
    pub compression: String,
    pub target: String,
    pub graph_digest: String,
    pub graph: SymbolGraph,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slice: Option<SymbolGraphSlice>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace_receipts: Vec<TraceReceipt>,
}

pub fn build_zjx_envelope(
    target: impl Into<String>,
    graph: SymbolGraph,
    slice: Option<SymbolGraphSlice>,
    trace_receipts: Vec<TraceReceipt>,
) -> SleyZjxEnvelope {
    let graph_bytes = serde_json::to_vec(&graph).expect("serialize symbol graph for ZJX digest");
    let graph_digest = content_digest(&graph_bytes);

    SleyZjxEnvelope {
        schema: "sley.zjx.envelope.v0".to_string(),
        format: "zjx-preview-json".to_string(),
        compression: "none".to_string(),
        target: target.into(),
        graph_digest,
        graph,
        slice,
        trace_receipts,
    }
}
