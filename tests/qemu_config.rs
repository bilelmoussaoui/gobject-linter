use clap::ValueEnum;
use gobject_linter::{
    config::{Config, RuleLevel},
    scanner::{RuleName, create_all_rules},
};

#[test]
fn namespaced_rule_name_works_in_toml_and_cli() {
    let config: Config = toml::from_str(
        r#"
        [rules]
        "qemu:coroutine_fn" = "error"
        "#,
    )
    .expect("namespaced QEMU config");

    assert_eq!(
        config
            .get_rule_config("qemu:coroutine_fn")
            .expect("QEMU rule config")
            .level,
        Some(RuleLevel::Error)
    );
    assert_eq!(
        RuleName::from_str("qemu:coroutine_fn", false)
            .expect("namespaced CLI value")
            .as_str(),
        "qemu:coroutine_fn"
    );
}

#[test]
fn qemu_rules_follow_the_base_registry() {
    let config = Config::default();
    let rules = create_all_rules(&config);
    assert_eq!(
        rules.last().expect("registered rules").rule.name(),
        "qemu:coroutine_fn"
    );
}
