#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostEffectContract {
    pub callee: &'static str,
    pub source_needle: &'static str,
    pub effects: &'static [&'static str],
}

pub const HOST_EFFECT_CONTRACTS: &[HostEffectContract] = &[
    HostEffectContract {
        callee: "fs.read_text",
        source_needle: "fs.read_text(",
        effects: &["FileRead"],
    },
    HostEffectContract {
        callee: "fs.try_read_text",
        source_needle: "fs.try_read_text(",
        effects: &["FileRead"],
    },
    HostEffectContract {
        callee: "fs.write_text",
        source_needle: "fs.write_text(",
        effects: &["FileWrite"],
    },
    HostEffectContract {
        callee: "fs.try_write_text",
        source_needle: "fs.try_write_text(",
        effects: &["FileWrite"],
    },
    HostEffectContract {
        callee: "db.query_one",
        source_needle: "db.query_one(",
        effects: &["DatabaseRead", "DbRead"],
    },
    HostEffectContract {
        callee: "db.query",
        source_needle: "db.query(",
        effects: &["DatabaseRead", "DbRead"],
    },
    HostEffectContract {
        callee: "db.try_query_one",
        source_needle: "db.try_query_one(",
        effects: &["DatabaseRead", "DbRead"],
    },
    HostEffectContract {
        callee: "db.try_query",
        source_needle: "db.try_query(",
        effects: &["DatabaseRead", "DbRead"],
    },
    HostEffectContract {
        callee: "db.try_insert",
        source_needle: "db.try_insert(",
        effects: &["DatabaseWrite", "DbWrite"],
    },
    HostEffectContract {
        callee: "model.try_complete",
        source_needle: "model.try_complete(",
        effects: &["ModelCall"],
    },
    HostEffectContract {
        callee: "http.try_get_text",
        source_needle: "http.try_get_text(",
        effects: &["Network"],
    },
    HostEffectContract {
        callee: "shell.try_run",
        source_needle: "shell.try_run(",
        effects: &["Shell"],
    },
    HostEffectContract {
        callee: "secrets.try_get",
        source_needle: "secrets.try_get(",
        effects: &["SecretRead"],
    },
    HostEffectContract {
        callee: "deploy.try_stage",
        source_needle: "deploy.try_stage(",
        effects: &["Deploy"],
    },
    HostEffectContract {
        callee: "spend.try_authorize",
        source_needle: "spend.try_authorize(",
        effects: &["Spend"],
    },
];

pub fn host_effect_contracts() -> &'static [HostEffectContract] {
    HOST_EFFECT_CONTRACTS
}

pub fn host_effects_for_callee(callee: &str) -> Option<&'static [&'static str]> {
    HOST_EFFECT_CONTRACTS
        .iter()
        .find(|contract| contract.callee == callee)
        .map(|contract| contract.effects)
}

pub fn is_host_callee_path(name: &str) -> bool {
    name.split('.').next().is_some_and(is_host_root)
}

pub fn is_host_root(name: &str) -> bool {
    matches!(
        name,
        "db" | "fs" | "http" | "shell" | "model" | "secrets" | "deploy" | "spend"
    )
}
