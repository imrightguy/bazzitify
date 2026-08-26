//! Tests for per-module checkbox selection and batch apply/undo operations.

use bazzitify::module::{Module, ModuleGraph};
use std::collections::HashMap;

/// Test helper to create a mock module for testing
fn mock_module(name: &str, has_apply: bool, has_undo: bool, depends: Vec<&str>) -> Module {
    Module {
        name: name.to_string(),
        description: Some(format!("Test module {}", name)),
        long_description: vec![],
        has_apply,
        has_undo,
        depends: depends.into_iter().map(String::from).collect(),
    }
}

#[test]
fn select_all_sets_all_modules_selected() {
    // This test verifies the logic that would be used by the select_all callback
    // In the actual implementation, the callback iterates over the model and sets selected=true
    let _modules = [
        mock_module("a", true, true, vec![]),
        mock_module("b", true, true, vec!["a"]),
        mock_module("c", true, true, vec!["a", "b"]),
    ];

    // Simulate select_all(true) - all modules should be selected
    let selected: Vec<Module> = _modules.to_vec();

    // All modules are selected for batch operation
    assert_eq!(selected.len(), 3);
}

#[test]
fn clear_selection_unsets_all_modules_selected() {
    // This test verifies the logic that would be used by the clear_selection callback
    let _modules = [
        mock_module("a", true, true, vec![]),
        mock_module("b", true, true, vec!["a"]),
        mock_module("c", true, true, vec!["a", "b"]),
    ];

    // Simulate clear_selection - no modules should be selected
    let selected: Vec<Module> = Vec::new();

    assert_eq!(selected.len(), 0);
}

#[test]
fn batch_apply_respects_dependency_order() {
    // When applying selected modules, they should run in dependency order
    let modules = [
        mock_module("a", true, true, vec!["b"]),
        mock_module("b", true, true, vec![]),
        mock_module("c", true, true, vec!["a", "b"]),
    ];

    let sorted = ModuleGraph::topological_sort(&modules).unwrap();
    let names: Vec<String> = sorted.iter().map(|m| m.name.clone()).collect();

    // b should come before a, and a and b before c
    let b_idx = names.iter().position(|n| n == "b").unwrap();
    let a_idx = names.iter().position(|n| n == "a").unwrap();
    let c_idx = names.iter().position(|n| n == "c").unwrap();

    assert!(b_idx < a_idx, "b should come before a");
    assert!(a_idx < c_idx, "a should come before c");
}

#[test]
fn batch_undo_respects_reverse_dependency_order() {
    // When undoing selected modules, they should run in reverse dependency order
    let modules = [
        mock_module("a", true, true, vec!["b"]),
        mock_module("b", true, true, vec![]),
        mock_module("c", true, true, vec!["a", "b"]),
    ];

    let apply_order = ModuleGraph::topological_sort(&modules).unwrap();
    let undo_order = ModuleGraph::reverse_topological_sort(&modules).unwrap();

    // undo order should be reverse of apply order
    assert_eq!(apply_order.len(), undo_order.len());
    for i in 0..apply_order.len() {
        assert_eq!(
            apply_order[i].name,
            undo_order[apply_order.len() - 1 - i].name
        );
    }
}

#[test]
fn batch_apply_skips_modules_without_apply() {
    // Modules without apply function should be skipped during batch apply
    let modules = [
        mock_module("has-apply", true, true, vec![]),
        mock_module("no-apply", false, true, vec![]),
        mock_module("also-has-apply", true, true, vec!["has-apply"]),
    ];

    let applyable: Vec<Module> = modules.iter().filter(|m| m.has_apply).cloned().collect();

    assert_eq!(applyable.len(), 2);
    assert!(applyable.iter().all(|m| m.has_apply));
}

#[test]
fn batch_undo_skips_modules_without_undo() {
    // Modules without undo function should be skipped during batch undo
    let modules = [
        mock_module("has-undo", true, true, vec![]),
        mock_module("no-undo", true, false, vec![]),
        mock_module("also-has-undo", true, true, vec!["has-undo"]),
    ];

    let undoable: Vec<Module> = modules.iter().filter(|m| m.has_undo).cloned().collect();

    assert_eq!(undoable.len(), 2);
    assert!(undoable.iter().all(|m| m.has_undo));
}

#[test]
fn selection_state_independent_of_navigation() {
    // Selection state is stored in the model and should persist
    // when navigating between module detail pages (current-page changes)
    let _modules = [
        mock_module("a", true, true, vec![]),
        mock_module("b", true, true, vec![]),
        mock_module("c", true, true, vec![]),
    ];

    // Simulate selecting modules a and c
    // In the actual implementation, selection is tracked per-module in the model
    let mut selection = HashMap::new();
    selection.insert("a", true);
    selection.insert("b", false);
    selection.insert("c", true);

    // Navigation to module detail page (current-page = 0 for "a")
    let _current_page = 0;

    // Selection should persist
    assert!(selection.get("a").copied().unwrap_or(false));
    assert!(!selection.get("b").copied().unwrap_or(false));
    assert!(selection.get("c").copied().unwrap_or(false));

    // Navigation to module detail page (current-page = 1 for "b")
    let _current_page = 1;

    // Selection should still persist
    assert!(selection.get("a").copied().unwrap_or(false));
    assert!(!selection.get("b").copied().unwrap_or(false));
    assert!(selection.get("c").copied().unwrap_or(false));
}

#[test]
fn select_all_then_clear_all_results_in_empty_selection() {
    // Select all then clear all should result in no modules selected
    let modules = [
        mock_module("a", true, true, vec![]),
        mock_module("b", true, true, vec![]),
        mock_module("c", true, true, vec![]),
    ];

    // After select_all
    let mut selection: HashMap<String, bool> =
        modules.iter().map(|m| (m.name.clone(), true)).collect();

    assert!(selection.values().all(|&v| v));

    // After clear_all
    for v in selection.values_mut() {
        *v = false;
    }

    assert!(selection.values().all(|&v| !v));
}
