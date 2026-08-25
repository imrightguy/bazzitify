//! Internationalization (i18n) support for bazzitify.
//!
//! Provides a simple translation system with locale support and argument interpolation.

use std::collections::HashMap;

/// Default locale used when no specific locale is set or as fallback.
pub const DEFAULT_LOCALE: &str = "en";

/// A map of translation keys to their localized strings for a single locale.
#[derive(Debug, Clone, Default)]
pub struct TranslationMap {
    locale: String,
    translations: HashMap<String, String>,
}

impl TranslationMap {
    /// Create a new empty translation map for the default locale.
    pub fn new() -> Self {
        Self {
            locale: DEFAULT_LOCALE.to_string(),
            translations: HashMap::new(),
        }
    }

    /// Create a new translation map for a specific locale.
    pub fn with_locale(locale: impl Into<String>) -> Self {
        Self {
            locale: locale.into(),
            translations: HashMap::new(),
        }
    }

    /// Get the locale of this translation map.
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.translations.is_empty()
    }

    /// Insert a translation key-value pair.
    pub fn insert(&mut self, key: String, value: String) {
        self.translations.insert(key, value);
    }

    /// Get a translation by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.translations.get(key).map(|s| s.as_str())
    }

    /// Get a translation with argument interpolation.
    /// Supports simple `{key}` placeholder replacement.
    pub fn get_with_args(&self, key: &str, args: &[(&str, &str)]) -> Option<String> {
        self.translations.get(key).map(|template| {
            let mut result = template.clone();
            for (k, v) in args {
                let placeholder = format!("{{{k}}}");
                result = result.replace(&placeholder, v);
            }
            result
        })
    }

    /// Get all translation keys.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.translations.keys()
    }

    /// Get the number of translations.
    pub fn len(&self) -> usize {
        self.translations.len()
    }
}

/// Translator manages multiple translation maps and handles locale fallback.
#[derive(Debug, Default)]
pub struct Translator {
    maps: HashMap<String, TranslationMap>,
    current_locale: String,
}

impl Translator {
    /// Create a new translator with the default locale.
    pub fn new() -> Self {
        Self {
            maps: HashMap::new(),
            current_locale: DEFAULT_LOCALE.to_string(),
        }
    }

    /// Get the current locale.
    pub fn locale(&self) -> &str {
        &self.current_locale
    }

    /// Set the current locale.
    pub fn set_locale(&mut self, locale: impl Into<String>) {
        self.current_locale = locale.into();
    }

    /// Add a translation for a specific locale.
    pub fn add_translation(
        &mut self,
        locale: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) {
        let locale = locale.into();
        let map = self
            .maps
            .entry(locale)
            .or_insert_with(|| TranslationMap::with_locale(&self.current_locale));
        map.insert(key.into(), value.into());
    }

    /// Add multiple translations for a locale from a TranslationMap.
    pub fn add_translations(&mut self, locale: impl Into<String>, map: TranslationMap) {
        self.maps.insert(locale.into(), map);
    }

    /// Translate a key using the current locale with fallback to default.
    pub fn t(&self, key: &str) -> String {
        // Try current locale first
        if let Some(value) = self.maps.get(&self.current_locale).and_then(|m| m.get(key)) {
            return value.to_string();
        }

        // Fallback to default locale
        if self.current_locale != DEFAULT_LOCALE
            && let Some(value) = self.maps.get(DEFAULT_LOCALE).and_then(|m| m.get(key))
        {
            return value.to_string();
        }

        // Key not found - return key itself for debugging
        key.to_string()
    }

    /// Translate a key with argument interpolation.
    pub fn t_with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        // Try current locale first
        if let Some(value) = self
            .maps
            .get(&self.current_locale)
            .and_then(|m| m.get_with_args(key, args))
        {
            return value;
        }

        // Fallback to default locale
        if self.current_locale != DEFAULT_LOCALE
            && let Some(value) = self
                .maps
                .get(DEFAULT_LOCALE)
                .and_then(|m| m.get_with_args(key, args))
        {
            return value;
        }

        // Key not found - return key itself for debugging
        key.to_string()
    }

    /// Check if a translation exists for the current locale (with fallback).
    pub fn has(&self, key: &str) -> bool {
        self.maps
            .get(&self.current_locale)
            .is_some_and(|m| m.get(key).is_some())
            || (self.current_locale != DEFAULT_LOCALE
                && self
                    .maps
                    .get(DEFAULT_LOCALE)
                    .is_some_and(|m| m.get(key).is_some()))
    }

    /// Get all available locales.
    pub fn available_locales(&self) -> Vec<String> {
        self.maps.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_map_creation() {
        let map = TranslationMap::new();
        assert_eq!(map.locale(), DEFAULT_LOCALE);
        assert!(map.is_empty());
    }

    #[test]
    fn translation_map_with_locale() {
        let map = TranslationMap::with_locale("fr");
        assert_eq!(map.locale(), "fr");
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

    #[test]
    fn translator_t_with_args() {
        let mut translator = Translator::new();
        translator.add_translation(
            "en".to_string(),
            "greeting".to_string(),
            "Hello, {name}!".to_string(),
        );

        translator.set_locale("en".to_string());
        assert_eq!(
            translator.t_with_args("greeting", &[("name", "Alice")]),
            "Hello, Alice!"
        );
    }

    #[test]
    fn translator_has() {
        let mut translator = Translator::new();
        translator.add_translation("en".to_string(), "key1".to_string(), "Value".to_string());

        assert!(translator.has("key1"));
        assert!(!translator.has("key2"));
    }
}
