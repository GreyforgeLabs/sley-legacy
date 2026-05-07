use serde::Serialize;

use crate::trace::{TRACE_SEAL_SCHEMA, TraceSeal};
use crate::verify::VerifyReport;
use crate::zjx::SleyZjxEnvelope;

pub const DEPLOY_ARTIFACTS_SCHEMA: &str = "sley.deploy.artifacts.v0";
pub const DEPLOY_REPORT_SCHEMA: &str = "sley.deploy.report.v0";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DeployReport {
    pub schema: String,
    pub status: String,
    pub mode: String,
    pub target: String,
    pub environment: String,
    pub policy: DeployPolicy,
    pub summary: DeploySummary,
    pub verify: VerifyReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seal: Option<TraceSeal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<DeployPackageSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<DeployArtifacts>,
    pub next_actions: Vec<DeployAction>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeployPolicy {
    pub live_deploy_allowed: bool,
    pub external_mutations: bool,
    pub provider_calls: bool,
    pub requires_operator_approval: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeploySummary {
    pub verify_status: String,
    pub seal_status: String,
    pub package_status: String,
    pub module_count: usize,
    pub task_count: usize,
    pub receipt_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seal_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeployPackageSummary {
    pub source_schema: String,
    pub format: String,
    pub compression: String,
    pub target: String,
    pub graph_digest: String,
    pub module_count: usize,
    pub trace_receipt_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeployArtifacts {
    pub directory: String,
    pub report: String,
    pub seal: String,
    pub package: String,
    pub manifest: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeployArtifactManifest {
    pub schema: String,
    pub target: String,
    pub environment: String,
    pub mode: String,
    pub policy: DeployPolicy,
    pub summary: DeployArtifactSummary,
    pub files: DeployArtifactFiles,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeployArtifactSummary {
    pub verify_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seal_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeployArtifactFiles {
    pub report: DeployArtifactFile,
    pub seal: DeployArtifactFile,
    pub package: DeployArtifactFile,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeployArtifactFile {
    pub path: String,
    pub schema: String,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeployAction {
    pub kind: String,
    pub reason: String,
    pub command: Vec<String>,
}

pub fn build_deploy_report(
    target: impl Into<String>,
    environment: impl Into<String>,
    verify: VerifyReport,
    seal: Option<TraceSeal>,
    package: Option<SleyZjxEnvelope>,
    artifacts: Option<DeployArtifacts>,
) -> DeployReport {
    let target = target.into();
    let environment = environment.into();
    let package = package.map(summarize_package);
    let ready = verify.status == "passed" && seal.is_some() && package.is_some();
    let summary = DeploySummary {
        verify_status: verify.status.clone(),
        seal_status: if seal.is_some() { "passed" } else { "skipped" }.to_string(),
        package_status: if package.is_some() {
            "passed"
        } else {
            "skipped"
        }
        .to_string(),
        module_count: seal.as_ref().map_or(0, |seal| seal.module_count),
        task_count: seal.as_ref().map_or(0, |seal| seal.task_count),
        receipt_count: seal.as_ref().map_or(0, |seal| seal.receipt_count),
        seal_digest: seal.as_ref().map(|seal| seal.seal_digest.clone()),
        graph_digest: package.as_ref().map(|package| package.graph_digest.clone()),
    };
    let next_actions = if ready {
        ready_actions(&target, artifacts.as_ref())
    } else {
        blocked_actions(&target)
    };
    DeployReport {
        schema: DEPLOY_REPORT_SCHEMA.to_string(),
        status: if ready { "ready" } else { "blocked" }.to_string(),
        mode: "dry_run".to_string(),
        target,
        environment,
        policy: DeployPolicy {
            live_deploy_allowed: false,
            external_mutations: false,
            provider_calls: false,
            requires_operator_approval: true,
        },
        summary,
        verify,
        seal,
        package,
        artifacts,
        next_actions,
    }
}

pub fn build_deploy_artifact_manifest(
    report: &DeployReport,
    artifacts: &DeployArtifacts,
    report_digest: impl Into<String>,
    seal_file_digest: impl Into<String>,
    package_file_digest: impl Into<String>,
) -> DeployArtifactManifest {
    let seal_schema = report
        .seal
        .as_ref()
        .map(|seal| seal.schema.clone())
        .unwrap_or_else(|| TRACE_SEAL_SCHEMA.to_string());
    let package_schema = report
        .package
        .as_ref()
        .map(|package| package.source_schema.clone())
        .unwrap_or_else(|| "sley.zjx.envelope.v0".to_string());

    DeployArtifactManifest {
        schema: DEPLOY_ARTIFACTS_SCHEMA.to_string(),
        target: report.target.clone(),
        environment: report.environment.clone(),
        mode: report.mode.clone(),
        policy: report.policy.clone(),
        summary: DeployArtifactSummary {
            verify_status: report.summary.verify_status.clone(),
            seal_digest: report.summary.seal_digest.clone(),
            graph_digest: report.summary.graph_digest.clone(),
        },
        files: DeployArtifactFiles {
            report: DeployArtifactFile {
                path: artifacts.report.clone(),
                schema: report.schema.clone(),
                digest: report_digest.into(),
            },
            seal: DeployArtifactFile {
                path: artifacts.seal.clone(),
                schema: seal_schema,
                digest: seal_file_digest.into(),
            },
            package: DeployArtifactFile {
                path: artifacts.package.clone(),
                schema: package_schema,
                digest: package_file_digest.into(),
            },
        },
    }
}

fn summarize_package(package: SleyZjxEnvelope) -> DeployPackageSummary {
    DeployPackageSummary {
        source_schema: package.schema,
        format: package.format,
        compression: package.compression,
        target: package.target,
        graph_digest: package.graph_digest,
        module_count: package.graph.modules.len(),
        trace_receipt_count: package.trace_receipts.len(),
    }
}

fn ready_actions(target: &str, artifacts: Option<&DeployArtifacts>) -> Vec<DeployAction> {
    let mut actions = vec![
        DeployAction {
            kind: "review_seal".to_string(),
            reason: "inspect the content-addressed seal before any live deployment decision"
                .to_string(),
            command: command(["sley", "seal", "--json", target]),
        },
        DeployAction {
            kind: "review_package".to_string(),
            reason: "inspect the ZJX preview envelope before operator deployment approval"
                .to_string(),
            command: command(["sley", "zjx", "--json", target]),
        },
    ];
    if let Some(artifacts) = artifacts {
        actions.push(DeployAction {
            kind: "inspect_deploy_artifacts".to_string(),
            reason: "validate the deploy handoff directory before operator deployment approval"
                .to_string(),
            command: command([
                "sley-contract",
                "inspect-deploy-artifacts",
                artifacts.directory.as_str(),
                "--json",
            ]),
        });
    }
    actions
}

fn blocked_actions(target: &str) -> Vec<DeployAction> {
    vec![
        DeployAction {
            kind: "repair_verify_gate".to_string(),
            reason: "strict verification must pass before a deploy package can be prepared"
                .to_string(),
            command: command(["sley", "verify", "--json", "--deny-warnings", target]),
        },
        DeployAction {
            kind: "inspect_readiness".to_string(),
            reason: "use doctor output to find diagnostics, lint findings, or runtime gate gaps"
                .to_string(),
            command: command(["sley", "doctor", "--json", "--deny-warnings", target]),
        },
    ]
}

fn command<const N: usize>(items: [&str; N]) -> Vec<String> {
    items.into_iter().map(str::to_string).collect()
}
