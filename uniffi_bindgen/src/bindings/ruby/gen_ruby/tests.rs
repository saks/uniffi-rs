use super::{crate_name_from_module_path, is_reserved_word, Config, RubyWrapper};
use crate::interface::ComponentInterface;
use std::collections::BTreeMap;
use uniffi_meta::NamespaceMetadata;

fn namespace(crate_name: &str, name: &str) -> NamespaceMetadata {
    NamespaceMetadata {
        crate_name: crate_name.to_string(),
        name: name.to_string(),
    }
}

fn ci_with_namespaces(
    udl: &str,
    crate_name: &str,
    namespaces: &[(&str, &str)],
) -> ComponentInterface {
    let mut ci = ComponentInterface::from_webidl(udl, crate_name).unwrap();
    let map = namespaces
        .iter()
        .map(|(c, n)| (c.to_string(), namespace(c, n)))
        .collect::<BTreeMap<_, _>>();
    ci.set_crate_to_namespace_map(map);
    ci
}

#[test]
fn when_reserved_word() {
    assert!(is_reserved_word("end"));
}

#[test]
fn when_not_reserved_word() {
    assert!(!is_reserved_word("ruby"));
}

#[test]
fn cdylib_name() {
    let config = Config::default();

    assert_eq!("uniffi", config.cdylib_name());

    let config = Config {
        cdylib_name: Some("todolist".to_string()),
        ..Default::default()
    };

    assert_eq!("todolist", config.cdylib_name());
}

#[test]
fn cdylib_path() {
    let config = Config::default();

    assert_eq!("", config.cdylib_path());
    assert!(!config.custom_cdylib_path());

    let config = Config {
        cdylib_path: Some("/foo/bar".to_string()),
        ..Default::default()
    };

    assert_eq!("/foo/bar", config.cdylib_path());
    assert!(config.custom_cdylib_path());
}

#[test]
fn crate_name_from_module_path_normalizes_hyphens() {
    assert_eq!(crate_name_from_module_path("my-crate"), "my_crate");
    assert_eq!(crate_name_from_module_path("my_crate"), "my_crate");
    assert_eq!(crate_name_from_module_path("my-crate::sub"), "my_crate");
    assert_eq!(crate_name_from_module_path("my_crate::sub"), "my_crate");
}

#[test]
fn hyphenated_config_key_matches_underscored_module_path() {
    let mut config = Config::default();
    config
        .external_packages
        .insert("my-crate".into(), "Custom".into());
    config.normalize_external_package_keys().unwrap();
    assert_eq!(
        config.external_package_name("my_crate::sub", Some("my_ns")),
        "Custom"
    );
}

#[test]
fn underscored_config_key_matches_hyphenated_udl_module_path() {
    let mut config = Config::default();
    config
        .external_packages
        .insert("my_crate".into(), "Custom".into());
    config.normalize_external_package_keys().unwrap();
    assert_eq!(
        config.external_package_name("my-crate", Some("my_ns")),
        "Custom"
    );
}

#[test]
fn unmapped_crate_falls_back_to_namespace() {
    let config = Config::default();
    assert_eq!(
        config.external_package_name("other_crate", Some("other_ns")),
        "OtherNs"
    );
}

#[test]
fn normalize_external_package_keys_rejects_conflicting_values() {
    let mut config = Config::default();
    config
        .external_packages
        .insert("my-crate".into(), "A".into());
    config
        .external_packages
        .insert("my_crate".into(), "B".into());
    let err = config.normalize_external_package_keys().unwrap_err();
    assert!(err.to_string().contains("conflicting"));
}

#[test]
fn normalize_external_package_keys_allows_duplicate_equivalent_keys() {
    let mut config = Config::default();
    config
        .external_packages
        .insert("my-crate".into(), "Custom".into());
    config
        .external_packages
        .insert("my_crate".into(), "Custom".into());
    config.normalize_external_package_keys().unwrap();
    assert_eq!(config.external_packages.get("my_crate").unwrap(), "Custom");
    assert!(!config.external_packages.contains_key("my-crate"));
}

#[test]
fn is_external_module_treats_hyphenated_name_as_same_crate() {
    let ci = ComponentInterface::new("foo_bar");
    let wrapper = RubyWrapper::new(Config::default(), &ci);
    assert!(!wrapper.is_external_module("foo-bar"));
    assert!(!wrapper.is_external_module("foo_bar"));
    assert!(!wrapper.is_external_module("foo-bar::sub"));
    assert!(wrapper.is_external_module("other_crate"));
    assert!(wrapper.is_external_module("other-crate"));
}

const TWO_TYPES_UDL: &str = r#"
    namespace consumer {
        TypeA get_a();
        TypeB get_b();
    };

    [External="crate_a"]
    typedef dictionary TypeA;

    [External="crate_b"]
    typedef dictionary TypeB;
"#;

#[test]
fn external_mixin_modules_collapses_two_types_from_same_crate() {
    let ci = ci_with_namespaces(
        r#"
        namespace consumer {
            TypeA get_a();
            TypeB get_b();
        };

        [External="crate_a"]
        typedef dictionary TypeA;

        [External="crate_a"]
        typedef dictionary TypeB;
        "#,
        "consumer",
        &[("consumer", "consumer"), ("crate_a", "ns_a")],
    );
    let mixins = RubyWrapper::new(Config::default(), &ci)
        .external_mixin_modules()
        .unwrap();
    assert_eq!(mixins.len(), 1);
    assert_eq!(mixins[0].module_name, "NsA");
    assert_eq!(mixins[0].require_path, "ns_a");
}

#[test]
fn external_mixin_modules_lists_each_crate() {
    let ci = ci_with_namespaces(
        TWO_TYPES_UDL,
        "consumer",
        &[
            ("consumer", "consumer"),
            ("crate_a", "ns_a"),
            ("crate_b", "ns_b"),
        ],
    );
    let mut mixins = RubyWrapper::new(Config::default(), &ci)
        .external_mixin_modules()
        .unwrap();
    mixins.sort_by(|a, b| a.require_path.cmp(&b.require_path));
    assert_eq!(mixins.len(), 2);
    assert_eq!(mixins[0].module_name, "NsA");
    assert_eq!(mixins[0].require_path, "ns_a");
    assert_eq!(mixins[1].module_name, "NsB");
    assert_eq!(mixins[1].require_path, "ns_b");
}

#[test]
fn external_mixin_modules_errors_on_camel_case_collision() {
    let ci = ci_with_namespaces(
        TWO_TYPES_UDL,
        "consumer",
        &[
            ("consumer", "consumer"),
            ("crate_a", "foo_bar"),
            ("crate_b", "fooBar"),
        ],
    );
    let err = RubyWrapper::new(Config::default(), &ci)
        .external_mixin_modules()
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("FooBar"), "{msg}");
    assert!(msg.contains("crate_a"), "{msg}");
    assert!(msg.contains("crate_b"), "{msg}");
}

#[test]
fn external_mixin_modules_errors_on_external_packages_collision() {
    let ci = ci_with_namespaces(
        TWO_TYPES_UDL,
        "consumer",
        &[
            ("consumer", "consumer"),
            ("crate_a", "ns_a"),
            ("crate_b", "ns_b"),
        ],
    );
    let mut config = Config::default();
    config
        .external_packages
        .insert("crate_a".into(), "Shared".into());
    config
        .external_packages
        .insert("crate_b".into(), "Shared".into());
    let err = RubyWrapper::new(config, &ci)
        .external_mixin_modules()
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("Shared"), "{msg}");
    assert!(msg.contains("crate_a"), "{msg}");
    assert!(msg.contains("crate_b"), "{msg}");
}

#[test]
fn external_mixin_modules_collapses_hyphenated_and_underscored_crate() {
    let ci = ci_with_namespaces(
        r#"
        namespace consumer {
            TypeA get_a();
            TypeB get_b();
        };

        [External="my-crate"]
        typedef dictionary TypeA;

        [External="my_crate"]
        typedef dictionary TypeB;
        "#,
        "consumer",
        &[("consumer", "consumer"), ("my_crate", "my_ns")],
    );
    let mixins = RubyWrapper::new(Config::default(), &ci)
        .external_mixin_modules()
        .unwrap();
    assert_eq!(mixins.len(), 1);
    assert_eq!(mixins[0].module_name, "MyNs");
    assert_eq!(mixins[0].require_path, "my_ns");
}
