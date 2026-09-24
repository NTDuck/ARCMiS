//! Envelope metadata test: the default registry registers every tool with
//! non-empty, consistent metadata.

use tools::envelope::CostClass;
use tools::envelope::SideEffectClass;
use tools::envelope::ToolCategory;
use tools::registry::default_registry;

#[test]
fn default_registry_registers_all_tools_with_consistent_metadata() {
    let registry = default_registry();
    assert_eq!(registry.len(), 17, "17 tools expected, got {}", registry.len());

    let expected: &[(&str, ToolCategory, SideEffectClass, CostClass)] = &[
        ("read", ToolCategory::FileSystem, SideEffectClass::ReadOnly, CostClass::Cheap),
        ("write", ToolCategory::FileSystem, SideEffectClass::WriteLocal, CostClass::Medium),
        ("edit", ToolCategory::FileSystem, SideEffectClass::WriteLocal, CostClass::Medium),
        ("search", ToolCategory::Search, SideEffectClass::ReadOnly, CostClass::Cheap),
        ("find", ToolCategory::Search, SideEffectClass::ReadOnly, CostClass::Cheap),
        ("ast_grep", ToolCategory::Search, SideEffectClass::ReadOnly, CostClass::Cheap),
        ("ast_edit", ToolCategory::Search, SideEffectClass::WriteLocal, CostClass::Medium),
        ("bash", ToolCategory::Runtime, SideEffectClass::Privileged, CostClass::Medium),
        ("eval", ToolCategory::Runtime, SideEffectClass::WriteRemote, CostClass::Expensive),
        ("ssh", ToolCategory::Runtime, SideEffectClass::WriteRemote, CostClass::Expensive),
        ("lsp", ToolCategory::CodeIntelligence, SideEffectClass::ReadOnly, CostClass::Medium),
        ("debug", ToolCategory::CodeIntelligence, SideEffectClass::Privileged, CostClass::Expensive),
        ("task", ToolCategory::Coordination, SideEffectClass::WriteLocal, CostClass::Expensive),
        ("irc", ToolCategory::Coordination, SideEffectClass::ReadOnly, CostClass::Cheap),
        ("todo", ToolCategory::Coordination, SideEffectClass::WriteLocal, CostClass::Cheap),
        ("job", ToolCategory::Coordination, SideEffectClass::ReadOnly, CostClass::Cheap),
        ("ask", ToolCategory::Coordination, SideEffectClass::ReadOnly, CostClass::Cheap),
    ];

    for (name, category, side_effect, cost) in expected {
        let meta = registry.metadata(name).unwrap_or_else(|| panic!("tool {name} missing from registry"));
        assert_eq!(meta.name, *name, "name mismatch for {name}");
        assert!(!meta.description.is_empty(), "empty description for {name}");
        assert_eq!(meta.category, *category, "category mismatch for {name}");
        assert_eq!(meta.side_effect, *side_effect, "side-effect mismatch for {name}");
        assert_eq!(meta.cost, *cost, "cost mismatch for {name}");
        assert!(meta.args_preview_cap > 0, "zero args cap for {name}");
        assert!(meta.result_preview_cap > 0, "zero result cap for {name}");
    }

    // Category queries return only tools of that category.
    let file_tools: Vec<String> =
        registry.by_category(ToolCategory::FileSystem).map(|tool| tool.metadata().name.to_owned()).collect();
    assert_eq!(file_tools.len(), 3, "filesystem tools: {file_tools:?}");

    // Side-effect queries partition the registry.
    let read_only = registry.by_side_effect(SideEffectClass::ReadOnly).count();
    assert!(read_only >= 5, "expected at least 5 read-only tools, got {read_only}");
}
