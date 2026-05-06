use std::fs;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value as JsonValue, json};

#[test]
fn scaffold_template_pack_creates_lint_clean_runnable_projects() {
    let cases = [
        TemplateCase {
            template: "library",
            expected_action: "verify_local",
            extra_source_path: None,
            gate_args: &[],
            expected_value: json!({"kind": "Text", "value": "sley library ready"}),
            deploy_ready: false,
        },
        TemplateCase {
            template: "cli",
            expected_action: "verify_local",
            extra_source_path: None,
            gate_args: &[],
            expected_value: json!({"kind": "Text", "value": "usage: sley-app"}),
            deploy_ready: false,
        },
        TemplateCase {
            template: "service-gate",
            expected_action: "verify_seeded_service",
            extra_source_path: None,
            gate_args: &[
                "--cap",
                "Network",
                "--http-text",
                "https://example.test/health",
                "service ready",
            ],
            expected_value: json!({
                "kind": "Ok",
                "value": {"kind": "Text", "value": "service service ready"}
            }),
            deploy_ready: false,
        },
        TemplateCase {
            template: "data-pipeline",
            expected_action: "verify_local",
            extra_source_path: None,
            gate_args: &[],
            expected_value: json!({"kind": "Int", "value": 14}),
            deploy_ready: false,
        },
        TemplateCase {
            template: "agent-task-pack",
            expected_action: "verify_seeded_agent",
            extra_source_path: None,
            gate_args: &[
                "--cap",
                "SecretRead",
                "--secret",
                "api_key",
                "redacted",
                "--cap",
                "Network",
                "--http-text",
                "https://example.test/profile",
                "profile ready",
                "--cap",
                "ModelCall",
                "--model-output",
                "deploy-plan",
                "plan approved",
                "--cap",
                "Deploy",
                "--deploy-result",
                "staging",
                "staged",
            ],
            expected_value: json!({
                "kind": "Ok",
                "value": {"kind": "Text", "value": "profile ready | plan approved | staged"}
            }),
            deploy_ready: true,
        },
        TemplateCase {
            template: "agent-project",
            expected_action: "verify_seeded_agent",
            extra_source_path: Some("src/app/pipeline.sley"),
            gate_args: &[
                "--cap",
                "SecretRead",
                "--secret",
                "api_key",
                "redacted",
                "--cap",
                "Network",
                "--http-text",
                "https://example.test/profile",
                "profile ready",
                "--cap",
                "ModelCall",
                "--model-output",
                "deploy-plan",
                "plan approved",
                "--cap",
                "Deploy",
                "--deploy-result",
                "staging",
                "staged",
            ],
            expected_value: json!({
                "kind": "Ok",
                "value": {"kind": "Text", "value": "profile ready | plan approved | staged"}
            }),
            deploy_ready: true,
        },
    ];

    for case in cases {
        let root = temp_project_dir(case.template);
        let project_name = format!("{}-app", case.template);
        let scaffold = sley_cmd()
            .args([
                "new",
                "--json",
                "--template",
                case.template,
                "--name",
                &project_name,
                "--module",
                "app.main",
            ])
            .arg(&root)
            .output()
            .unwrap_or_else(|error| panic!("run sley new {}: {error}", case.template));
        let scaffold_stdout =
            String::from_utf8(scaffold.stdout).expect("scaffold stdout should be utf8");
        let scaffold_stderr =
            String::from_utf8(scaffold.stderr).expect("scaffold stderr should be utf8");
        assert!(
            scaffold.status.success(),
            "sley new {} failed\nstdout: {scaffold_stdout}\nstderr: {scaffold_stderr}",
            case.template
        );
        let scaffold_json: JsonValue =
            serde_json::from_str(&scaffold_stdout).expect("parse scaffold JSON");
        assert_eq!(
            scaffold_json.pointer("/schema"),
            Some(&json!("sley.project.scaffold.v0"))
        );
        assert_eq!(
            scaffold_json.pointer("/project/template"),
            Some(&json!(case.template))
        );
        assert!(
            scaffold_json
                .pointer("/next_actions")
                .and_then(JsonValue::as_array)
                .expect("next_actions")
                .iter()
                .any(|action| action.pointer("/kind") == Some(&json!(case.expected_action))),
            "scaffold {} missing expected next action {}",
            case.template,
            case.expected_action
        );
        assert!(root.join("sley.toml").exists());
        assert!(root.join("README.md").exists());
        assert!(root.join("src/app/main.sley").exists());
        if let Some(extra_source_path) = case.extra_source_path {
            assert!(root.join(extra_source_path).exists());
            assert_eq!(
                scaffold_json.pointer("/files/3/path"),
                Some(&json!(extra_source_path))
            );
        }

        run_success(&root, &["check", "--json", "."]);
        run_success(&root, &["lint", "--json", "--deny-warnings", "."]);

        let mut verify_args = vec!["verify", "--json", "--deny-warnings"];
        verify_args.extend(case.gate_args);
        verify_args.push(".");
        run_success(&root, &verify_args);

        let mut run_args = vec!["run", "--json"];
        run_args.extend(case.gate_args);
        run_args.push(".");
        let run_json = run_success(&root, &run_args);
        assert_eq!(run_json.pointer("/status"), Some(&json!("passed")));
        assert_eq!(run_json.get("value"), Some(&case.expected_value));

        if case.deploy_ready {
            let mut deploy_args = vec![
                "deploy",
                "--json",
                "--dry-run",
                "--artifacts-dir",
                ".sley/deploy",
            ];
            deploy_args.extend(case.gate_args);
            deploy_args.push(".");
            let deploy_json = run_success(&root, &deploy_args);
            assert_eq!(
                deploy_json.pointer("/schema"),
                Some(&json!("sley.deploy.report.v0"))
            );
            assert_eq!(deploy_json.pointer("/status"), Some(&json!("ready")));
            assert!(root.join(".sley/deploy/deploy-report.json").exists());
        }

        let _ = fs::remove_dir_all(root);
    }
}

struct TemplateCase {
    template: &'static str,
    expected_action: &'static str,
    extra_source_path: Option<&'static str>,
    gate_args: &'static [&'static str],
    expected_value: JsonValue,
    deploy_ready: bool,
}

fn run_success(root: &PathBuf, args: &[&str]) -> JsonValue {
    let output = sley_cmd()
        .current_dir(root)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("run sley {}: {error}", args.join(" ")));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        output.status.success(),
        "sley {} failed\nstdout: {stdout}\nstderr: {stderr}",
        args.join(" ")
    );
    serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("parse sley {} JSON: {error}\n{stdout}", args.join(" ")))
}

fn sley_cmd() -> ProcessCommand {
    ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
}

fn temp_project_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "sley-template-{name}-{}-{timestamp}",
        std::process::id()
    ))
}
