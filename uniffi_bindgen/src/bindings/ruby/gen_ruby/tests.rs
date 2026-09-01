use super::{crate_name_from_module_path, is_reserved_word, Config, RubyWrapper};
use crate::interface::ComponentInterface;

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
