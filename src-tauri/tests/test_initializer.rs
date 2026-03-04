/// Integration test: scan the current project (ai_doc_manager) and verify
/// that tech_detector + project_initializer produce sensible output.

#[test]
fn test_scan_current_project() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    println!("Scanning project root: {}", root.display());

    let scan = docguardian_lib::services::tech_detector::scan_project(root)
        .expect("scan should succeed");

    // Should detect Rust + TypeScript
    println!("Languages: {:?}", scan.languages);
    assert!(
        scan.languages.iter().any(|l| matches!(l, docguardian_lib::services::tech_detector::Language::Rust)),
        "Should detect Rust"
    );
    assert!(
        scan.languages.iter().any(|l| matches!(l, docguardian_lib::services::tech_detector::Language::TypeScript)),
        "Should detect TypeScript"
    );

    // Should detect Tauri + React frameworks
    println!("Frameworks: {:?}", scan.frameworks);
    assert!(
        scan.frameworks.iter().any(|f| matches!(f, docguardian_lib::services::tech_detector::Framework::Tauri)),
        "Should detect Tauri"
    );
    assert!(
        scan.frameworks.iter().any(|f| matches!(f, docguardian_lib::services::tech_detector::Framework::React)),
        "Should detect React"
    );

    // Should detect existing AGENTS.md
    println!("AI governance: agents_md={:?}", scan.ai_governance.agents_md);

    // Git may or may not be initialized
    println!("Git stats: is_repo={}, {} commits, {} fixes",
        scan.git_stats.is_git_repo, scan.git_stats.total_commits, scan.git_stats.fix_commits);

    // Directory structure
    println!("Top dirs: {:?}", scan.dir_structure.top_level_dirs);
    assert!(scan.dir_structure.has_src, "Should have src directory");

    println!("Tools: {:?}", scan.tools.iter().map(|t| &t.name).collect::<Vec<_>>());
    println!("Project name: {}", scan.project_name);

    // Now test plan generation
    let plan = docguardian_lib::core::project_initializer::generate_init_plan(&scan);

    println!("\n=== Init Plan ===");
    println!("Mode: {:?}", plan.mode);
    println!("Rules ({}):", plan.rules.len());
    for rule in &plan.rules {
        println!("  [{}] {} (accepted={})", rule.id, rule.content, rule.accepted);
    }
    println!("Files ({}):", plan.files.len());
    for file in &plan.files {
        println!("  {} — {} (overwrite={})", file.rel_path, file.description, file.overwrite);
    }

    // Should generate rules
    assert!(!plan.rules.is_empty(), "Should generate at least some rules");

    // Should generate files
    assert!(!plan.files.is_empty(), "Should generate at least some files");
    assert!(
        plan.files.iter().any(|f| f.rel_path == "AGENTS.md"),
        "Should include AGENTS.md"
    );
    assert!(
        plan.files.iter().any(|f| f.rel_path == ".docguardian.toml"),
        "Should include .docguardian.toml"
    );

    // Print AGENTS.md content preview
    if let Some(agents) = plan.files.iter().find(|f| f.rel_path == "AGENTS.md") {
        println!("\n=== AGENTS.md Preview (first 500 chars) ===");
        let preview: String = agents.content.chars().take(500).collect();
        println!("{}", preview);
    }
}
