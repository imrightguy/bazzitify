//! Tests for the i18n (internationalization) module.

use bazzitify::i18n::{DEFAULT_LOCALE, TranslationMap, Translator};

#[test]
fn translation_map_creation() {
    let map = TranslationMap::new();
    assert_eq!(map.locale(), DEFAULT_LOCALE);
    assert!(map.is_empty());
}

#[test]
fn translation_map_add_and_get() {
    let mut map = TranslationMap::new();
    map.insert("welcome".to_string(), "Welcome to bazzitify".to_string());
    map.insert("apply_selected".to_string(), "Apply Selected".to_string());

    assert_eq!(map.get("welcome"), Some("Welcome to bazzitify"));
    assert_eq!(map.get("apply_selected"), Some("Apply Selected"));
    assert_eq!(map.get("nonexistent"), None);
}

#[test]
fn translation_map_with_args() {
    let mut map = TranslationMap::new();
    map.insert("greeting".to_string(), "Hello, {name}!".to_string());

    let result = map.get_with_args("greeting", &[("name", "World")]);
    assert_eq!(result, Some("Hello, World!".to_string()));
}

#[test]
fn translator_default_locale() {
    let translator = Translator::new();
    assert_eq!(translator.locale(), DEFAULT_LOCALE);
}

#[test]
fn translator_fallback_to_default() {
    let mut translator = Translator::new();
    translator.add_translation(DEFAULT_LOCALE, "key1".to_string(), "Default".to_string());
    // No translation for "fr" locale
    translator.set_locale("fr".to_string());

    // Should fall back to default locale
    assert_eq!(translator.t("key1"), "Default");
}

#[test]
fn translator_with_specific_locale() {
    let mut translator = Translator::new();
    translator.add_translation(
        "en".to_string(),
        "welcome".to_string(),
        "Welcome".to_string(),
    );
    translator.add_translation(
        "fr".to_string(),
        "welcome".to_string(),
        "Bienvenue".to_string(),
    );

    translator.set_locale("fr".to_string());
    assert_eq!(translator.t("welcome"), "Bienvenue");

    translator.set_locale("en".to_string());
    assert_eq!(translator.t("welcome"), "Welcome");
}

#[test]
fn translator_missing_key_returns_key() {
    let translator = Translator::new();
    // Missing key should return the key itself (for debugging)
    assert_eq!(translator.t("missing.key"), "missing.key");
}

/// Build-time validation: ensure all @tr("key") calls in app.slint have corresponding
/// entries in the translation .po file.
/// This test runs at compile time to catch missing translations early.
#[test]
fn distro_picker_localizes_its_user_facing_fallback() {
    use std::fs;
    use std::path::Path;

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let slint_path = Path::new(manifest_dir).join("ui").join("app.slint");
    let po_path = Path::new(manifest_dir)
        .join("translations")
        .join("en")
        .join("LC_MESSAGES")
        .join("bazzitify.po");

    let slint_content = fs::read_to_string(&slint_path).expect("Failed to read app.slint");
    let po_content = fs::read_to_string(&po_path).expect("Failed to read bazzitify.po");

    assert!(
        slint_content.contains("@tr(\"wizard-distro-other\")"),
        "the distro picker must not leave its user-facing fallback as a raw Slint string"
    );
    assert!(
        po_content.contains("msgid \"wizard-distro-other\""),
        "the distro picker fallback must be present in the English gettext catalog"
    );
}

#[test]
fn translation_keys_complete_in_po_file() {
    use std::fs;
    use std::path::Path;

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let slint_path = Path::new(manifest_dir).join("ui").join("app.slint");
    let po_path = Path::new(manifest_dir)
        .join("translations")
        .join("en")
        .join("LC_MESSAGES")
        .join("bazzitify.po");

    let slint_content = fs::read_to_string(&slint_path).expect("Failed to read app.slint");
    let po_content = fs::read_to_string(&po_path).expect("Failed to read bazzitify.po");

    // Extract all @tr("key") patterns from the Slint file
    let tr_regex = regex::Regex::new(r#"@tr\("([^"]+)"\)"#).unwrap();
    let slint_keys: std::collections::HashSet<_> = tr_regex
        .captures_iter(&slint_content)
        .map(|cap| cap[1].to_string())
        .collect();

    // Extract all msgid keys from the .po file
    let msgid_regex = regex::Regex::new(r#"msgid\s+"([^"]+)"#).unwrap();
    let po_keys: std::collections::HashSet<_> = msgid_regex
        .captures_iter(&po_content)
        .map(|cap| cap[1].to_string())
        .collect();

    // Find keys in Slint but missing from .po
    let missing_keys: Vec<_> = slint_keys.difference(&po_keys).cloned().collect();

    if !missing_keys.is_empty() {
        let mut msg =
            String::from("The following @tr() keys in app.slint are missing from bazzitify.po:\n");
        for key in &missing_keys {
            msg.push_str(&format!("  - {}\n", key));
        }
        msg.push_str(
            "\nAdd them to translations/en/LC_MESSAGES/bazzitify.po and recompile with msgfmt.",
        );
        panic!("{}", msg);
    }

    // Also check for orphaned keys in .po that aren't used in Slint (warning only)
    let orphaned_keys: Vec<_> = po_keys.difference(&slint_keys).cloned().collect();
    if !orphaned_keys.is_empty() {
        eprintln!("WARNING: The following keys exist in .po but are not used in app.slint:");
        for key in &orphaned_keys {
            eprintln!("  - {}", key);
        }
    }

    println!(
        "Translation validation passed: {} keys verified in both app.slint and .po",
        slint_keys.len()
    );
}
