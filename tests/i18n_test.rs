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
