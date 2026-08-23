use bazzitify::profile::{Profile, ProfileError};
use std::fs;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "bazzitify-profile-test-{}-{}",
        std::process::id(),
        name
    ))
}

#[test]
fn profile_serializes_with_modules_and_metadata() {
    let profile = Profile {
        version: 1,
        distro: "CachyOS".into(),
        date: "2026-08-23".into(),
        modules: vec![
            "sysctl".into(),
            "gpu-drivers".into(),
            "gaming-packages".into(),
        ],
    };
    let toml = profile.to_toml().unwrap();
    assert!(toml.contains("version = 1"));
    assert!(toml.contains("distro = \"CachyOS\""));
    assert!(toml.contains("date = \"2026-08-23\""));
    // serde serializes arrays in multiline format
    assert!(toml.contains("modules = ["));
    assert!(toml.contains("\"sysctl\""));
    assert!(toml.contains("\"gpu-drivers\""));
    assert!(toml.contains("\"gaming-packages\""));
}

#[test]
fn profile_deserializes_from_valid_toml() {
    let toml = r#"
version = 1
distro = "Arch Linux"
date = "2026-08-23"
modules = ["sysctl", "gpu-drivers", "gaming-packages"]
"#;
    let profile = Profile::from_toml(toml).unwrap();
    assert_eq!(profile.version, 1);
    assert_eq!(profile.distro, "Arch Linux");
    assert_eq!(profile.date, "2026-08-23");
    assert_eq!(
        profile.modules,
        vec!["sysctl", "gpu-drivers", "gaming-packages"]
    );
}

#[test]
fn profile_rejects_missing_version() {
    let toml = r#"
distro = "Arch"
date = "2026-08-23"
modules = ["sysctl"]
"#;
    let err = Profile::from_toml(toml).unwrap_err();
    assert!(matches!(err, ProfileError::MissingField { .. }));
    assert!(err.to_string().contains("version"));
}

#[test]
fn profile_rejects_missing_modules() {
    let toml = r#"
version = 1
distro = "Arch"
date = "2026-08-23"
"#;
    let err = Profile::from_toml(toml).unwrap_err();
    assert!(matches!(err, ProfileError::MissingField { .. }));
    assert!(err.to_string().contains("modules"));
}

#[test]
fn profile_rejects_unknown_version() {
    let toml = r#"
version = 999
distro = "Arch"
date = "2026-08-23"
modules = ["sysctl"]
"#;
    let err = Profile::from_toml(toml).unwrap_err();
    assert!(matches!(err, ProfileError::UnsupportedVersion { .. }));
}

#[test]
fn profile_validates_modules_exist_locally() {
    let dir = temp_dir("validate");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("sysctl.sh"), "# desc: t\nmodule_apply() { :; }\n").unwrap();
    fs::write(
        dir.join("gpu-drivers.sh"),
        "# desc: t\nmodule_apply() { :; }\n",
    )
    .unwrap();

    let profile = Profile {
        version: 1,
        distro: "Arch".into(),
        date: "2026-08-23".into(),
        modules: vec!["sysctl".into(), "gpu-drivers".into(), "nonexistent".into()],
    };

    let result = profile.validate_modules(&dir);
    assert!(result.is_ok()); // validates but warns
    let warnings = result.unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("nonexistent"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn profile_export_creates_file_in_config_dir() {
    let dir = temp_dir("export");
    fs::create_dir_all(&dir).unwrap();

    let profile = Profile {
        version: 1,
        distro: "CachyOS".into(),
        date: "2026-08-23".into(),
        modules: vec!["sysctl".into(), "gpu-drivers".into()],
    };

    let config_dir = dir.join("config").join("bazzitify").join("profiles");
    fs::create_dir_all(&config_dir).unwrap();

    let file_path = profile.export(&config_dir, "my-gaming").unwrap();
    assert!(file_path.exists());
    assert!(file_path.to_string_lossy().ends_with("my-gaming.toml"));

    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("modules = ["));
    assert!(content.contains("\"sysctl\""));
    assert!(content.contains("\"gpu-drivers\""));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn profile_import_loads_and_validates() {
    let dir = temp_dir("import");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("sysctl.sh"), "# desc: t\nmodule_apply() { :; }\n").unwrap();
    fs::write(
        dir.join("gpu-drivers.sh"),
        "# desc: t\nmodule_apply() { :; }\n",
    )
    .unwrap();

    let config_dir = dir.join("config").join("bazzitify").join("profiles");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
version = 1
distro = "CachyOS"
date = "2026-08-23"
modules = ["sysctl", "gpu-drivers", "missing-module"]
"#;
    let import_file = config_dir.join("import-test.toml");
    fs::write(&import_file, toml).unwrap();

    let (profile, warnings) = Profile::import(&import_file, &dir).unwrap();
    assert_eq!(
        profile.modules,
        vec!["sysctl", "gpu-drivers", "missing-module"]
    );
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("missing-module"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn profile_import_rejects_invalid_toml() {
    let dir = temp_dir("import-invalid");
    fs::create_dir_all(&dir).unwrap();

    let import_file = dir.join("bad.toml");
    fs::write(&import_file, "not valid toml [[[").unwrap();

    let err = Profile::import(&import_file, &dir).unwrap_err();
    assert!(matches!(err, ProfileError::TomlParse(_)));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn profile_list_returns_available_profiles() {
    let dir = temp_dir("list");
    fs::create_dir_all(&dir).unwrap();

    let config_dir = dir.join("config").join("bazzitify").join("profiles");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("profile1.toml"),
        "version = 1\nmodules = []\n",
    )
    .unwrap();
    fs::write(
        config_dir.join("profile2.toml"),
        "version = 1\nmodules = []\n",
    )
    .unwrap();
    fs::write(config_dir.join("not-a-profile.txt"), "ignore").unwrap();

    let profiles = Profile::list_profiles(&config_dir).unwrap();
    assert_eq!(profiles.len(), 2);
    assert!(profiles.contains(&"profile1".into()));
    assert!(profiles.contains(&"profile2".into()));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn profile_default_config_dir_follows_xdg() {
    // Just ensure the function exists and returns a path
    let path = Profile::default_config_dir();
    assert!(path.to_string_lossy().contains("bazzitify"));
    assert!(path.to_string_lossy().contains("profiles"));
}
