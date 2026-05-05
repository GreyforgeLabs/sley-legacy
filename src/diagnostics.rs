use serde::{Deserialize, Serialize};

pub const DIAGNOSTIC_REPORT_SCHEMA: &str = "sley.diagnostics.report.v0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceSpan {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepairHint {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub id: String,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repair_hints: Vec<RepairHint>,
}

impl Diagnostic {
    pub fn error(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            severity: Severity::Error,
            message: message.into(),
            node: None,
            span: None,
            repair_hints: Vec::new(),
        }
    }

    pub fn warning(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            severity: Severity::Warning,
            message: message.into(),
            node: None,
            span: None,
            repair_hints: Vec::new(),
        }
    }

    pub fn with_node(mut self, node: impl Into<String>) -> Self {
        self.node = Some(node.into());
        self
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_repair_hint(mut self, hint: RepairHint) -> Self {
        self.repair_hints.push(hint);
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticReport {
    pub schema: String,
    pub status: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticReport {
    pub fn from_diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        let status = if diagnostics.iter().any(Diagnostic::is_error) {
            "error"
        } else {
            "ok"
        };
        Self {
            schema: DIAGNOSTIC_REPORT_SCHEMA.to_string(),
            status: status.to_string(),
            diagnostics,
        }
    }
}
