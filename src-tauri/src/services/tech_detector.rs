use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DetectorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path does not exist: {0}")]
    PathNotFound(String),
}

/// Detected programming language
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Swift,
    Kotlin,
    CSharp,
    Ruby,
    Php,
    Other(String),
}

/// Detected framework
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Framework {
    React,
    Vue,
    Angular,
    Svelte,
    NextJs,
    NuxtJs,
    Express,
    Fastify,
    Django,
    Flask,
    FastApi,
    Axum,
    Actix,
    Tauri,
    Electron,
    Gin,
    Echo,
    Fiber,
    Chi,
    Kratos,
    Spring,
    Quarkus,
    Micronaut,
    Rails,
    Laravel,
    SwiftUI,
    UIKit,
    JetpackCompose,
    Flutter,
    Dotnet,
    Other(String),
}

/// Detected linter/tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub name: String,
    pub config_file: String,
    pub rules: Vec<String>,
}

/// Existing AI governance files found in the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiGovernanceFiles {
    pub agents_md: Option<PathBuf>,
    pub cursorrules: Option<PathBuf>,
    pub windsurfrules: Option<PathBuf>,
    pub progress_md: Option<PathBuf>,
    pub ai_dir: Option<PathBuf>,
}

/// Existing documentation found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistingDocs {
    pub readme: Option<PathBuf>,
    pub contributing: Option<PathBuf>,
    pub changelog: Option<PathBuf>,
    pub docs_dir: Option<PathBuf>,
    pub doc_files: Vec<PathBuf>,
}

/// Git statistics for the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStats {
    pub is_git_repo: bool,
    pub total_commits: u32,
    pub fix_commits: u32,
    pub revert_commits: u32,
    pub recent_fix_patterns: Vec<String>,
}

/// Dependency information extracted from package managers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub npm_deps: Vec<DepEntry>,
    pub npm_dev_deps: Vec<DepEntry>,
    pub cargo_deps: Vec<DepEntry>,
    pub go_deps: Vec<DepEntry>,
    pub python_deps: Vec<DepEntry>,
    pub java_deps: Vec<DepEntry>,
    pub swift_deps: Vec<DepEntry>,
    pub ruby_deps: Vec<DepEntry>,
    pub php_deps: Vec<DepEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepEntry {
    pub name: String,
    pub version: String,
}

/// CI/CD configuration summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiConfig {
    pub provider: String,
    pub workflows: Vec<CiWorkflow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiWorkflow {
    pub name: String,
    pub triggers: Vec<String>,
    pub steps_summary: Vec<String>,
}

/// Directory structure summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirStructure {
    pub top_level_dirs: Vec<String>,
    pub has_src: bool,
    pub has_tests: bool,
    pub has_ci: bool,
    pub total_files: u32,
    pub total_lines_code: u32,
    pub total_lines_docs: u32,
}

/// Complete scan result from tech detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechScanResult {
    pub root_path: String,
    pub project_name: String,
    pub languages: Vec<Language>,
    pub frameworks: Vec<Framework>,
    pub tools: Vec<ToolConfig>,
    pub ai_governance: AiGovernanceFiles,
    pub existing_docs: ExistingDocs,
    pub git_stats: GitStats,
    pub dir_structure: DirStructure,
    pub dependencies: DependencyInfo,
    pub ci_config: Option<CiConfig>,
}

/// Scan a project directory and detect its tech stack deterministically.
pub fn scan_project(root: &Path) -> Result<TechScanResult, DetectorError> {
    if !root.exists() {
        return Err(DetectorError::PathNotFound(root.display().to_string()));
    }

    let project_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let languages = detect_languages(root);
    let frameworks = detect_frameworks(root);
    let tools = detect_tools(root);
    let ai_governance = detect_ai_governance(root);
    let existing_docs = detect_existing_docs(root);
    let git_stats = detect_git_stats(root);
    let dir_structure = analyze_dir_structure(root);

    let dependencies = detect_dependencies(root);
    let ci_config = detect_ci_config(root);

    Ok(TechScanResult {
        root_path: root.display().to_string(),
        project_name,
        languages,
        frameworks,
        tools,
        ai_governance,
        existing_docs,
        git_stats,
        dir_structure,
        dependencies,
        ci_config,
    })
}

// --- Language detection ---

fn detect_languages(root: &Path) -> Vec<Language> {
    let mut langs = Vec::new();

    // Check both root and common subdirectories (e.g. src-tauri/Cargo.toml)
    let markers: &[(&[&str], Language)] = &[
        (&["Cargo.toml", "src-tauri/Cargo.toml"], Language::Rust),
        (&["package.json"], Language::TypeScript), // will refine below
        (&["requirements.txt", "pyproject.toml"], Language::Python),
        (&["go.mod"], Language::Go),
        (&["pom.xml", "build.gradle"], Language::Java),
        (&["Package.swift"], Language::Swift),
        (&["build.gradle.kts"], Language::Kotlin),
        (&["Gemfile"], Language::Ruby),
        (&["composer.json"], Language::Php),
    ];

    for (files, lang) in markers {
        for file in *files {
            if root.join(file).exists() {
                if !langs.contains(lang) {
                    langs.push(lang.clone());
                }
                break;
            }
        }
    }

    // Distinguish TypeScript vs JavaScript
    if root.join("package.json").exists() {
        let has_ts = root.join("tsconfig.json").exists()
            || root.join("tsconfig.app.json").exists();
        // Remove the generic TypeScript entry and add the correct one
        langs.retain(|l| !matches!(l, Language::TypeScript | Language::JavaScript));
        if has_ts {
            langs.push(Language::TypeScript);
        } else {
            langs.push(Language::JavaScript);
        }
    }

    // Check for .csproj files (C#)
    if has_file_with_extension(root, "csproj", 1) {
        langs.push(Language::CSharp);
    }

    langs
}

// --- Framework detection ---

fn detect_frameworks(root: &Path) -> Vec<Framework> {
    let mut frameworks = Vec::new();

    // Tauri
    if root.join("src-tauri").exists() || root.join("tauri.conf.json").exists() {
        frameworks.push(Framework::Tauri);
    }

    // Electron
    if root.join("electron").exists() || root.join("electron-builder.yml").exists() {
        frameworks.push(Framework::Electron);
    }

    // Read package.json for JS/TS frameworks
    if let Some(deps) = read_package_json_deps(root) {
        let dep_checks: &[(&str, Framework)] = &[
            ("react", Framework::React),
            ("vue", Framework::Vue),
            ("@angular/core", Framework::Angular),
            ("svelte", Framework::Svelte),
            ("next", Framework::NextJs),
            ("nuxt", Framework::NuxtJs),
            ("express", Framework::Express),
            ("fastify", Framework::Fastify),
        ];
        for (dep, fw) in dep_checks {
            if deps.contains_key(*dep) && !frameworks.contains(fw) {
                frameworks.push(fw.clone());
            }
        }
    }

    // Read Cargo.toml for Rust frameworks
    if let Ok(content) = std::fs::read_to_string(root.join("src-tauri/Cargo.toml"))
        .or_else(|_| std::fs::read_to_string(root.join("Cargo.toml")))
    {
        let rust_checks: &[(&str, Framework)] = &[
            ("axum", Framework::Axum),
            ("actix-web", Framework::Actix),
        ];
        for (dep, fw) in rust_checks {
            if content.contains(dep) && !frameworks.contains(fw) {
                frameworks.push(fw.clone());
            }
        }
    }

    // Python frameworks
    if let Ok(content) = std::fs::read_to_string(root.join("requirements.txt")) {
        let py_checks: &[(&str, Framework)] = &[
            ("django", Framework::Django),
            ("flask", Framework::Flask),
            ("fastapi", Framework::FastApi),
        ];
        for (dep, fw) in py_checks {
            if content.to_lowercase().contains(dep) && !frameworks.contains(fw) {
                frameworks.push(fw.clone());
            }
        }
    }

    // Go frameworks
    if let Ok(content) = std::fs::read_to_string(root.join("go.mod")) {
        let go_checks: &[(&str, Framework)] = &[
            ("gin-gonic/gin", Framework::Gin),
            ("labstack/echo", Framework::Echo),
            ("gofiber/fiber", Framework::Fiber),
            ("go-chi/chi", Framework::Chi),
            ("go-kratos/kratos", Framework::Kratos),
        ];
        for (dep, fw) in go_checks {
            if content.contains(dep) && !frameworks.contains(fw) {
                frameworks.push(fw.clone());
            }
        }
    }

    // Java frameworks (pom.xml / build.gradle)
    for java_file in &["pom.xml", "build.gradle", "build.gradle.kts"] {
        if let Ok(content) = std::fs::read_to_string(root.join(java_file)) {
            if content.contains("spring-boot") || content.contains("springframework") {
                if !frameworks.contains(&Framework::Spring) {
                    frameworks.push(Framework::Spring);
                }
            }
            if content.contains("quarkus") {
                if !frameworks.contains(&Framework::Quarkus) {
                    frameworks.push(Framework::Quarkus);
                }
            }
            if content.contains("micronaut") {
                if !frameworks.contains(&Framework::Micronaut) {
                    frameworks.push(Framework::Micronaut);
                }
            }
            // Android / Jetpack Compose
            if content.contains("com.android") || content.contains("android {") {
                if content.contains("compose") {
                    if !frameworks.contains(&Framework::JetpackCompose) {
                        frameworks.push(Framework::JetpackCompose);
                    }
                }
            }
        }
    }

    // Ruby
    if let Ok(content) = std::fs::read_to_string(root.join("Gemfile")) {
        if content.contains("rails") {
            frameworks.push(Framework::Rails);
        }
    }

    // PHP
    if let Ok(content) = std::fs::read_to_string(root.join("composer.json")) {
        if content.contains("laravel") {
            frameworks.push(Framework::Laravel);
        }
    }

    // Swift / iOS
    if let Ok(content) = std::fs::read_to_string(root.join("Package.swift")) {
        if content.contains("SwiftUI") {
            frameworks.push(Framework::SwiftUI);
        }
    }
    // Check for Xcode project with SwiftUI/UIKit
    if root.join("*.xcodeproj").exists() || root.join("*.xcworkspace").exists()
        || has_file_with_extension(root, "xcodeproj", 1)
    {
        // Heuristic: check for SwiftUI usage in source files
        let swift_src = root.join("Sources");
        let swift_src_alt = root.join("src");
        for src_dir in &[&swift_src, &swift_src_alt, &root.to_path_buf()] {
            if src_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(src_dir) {
                    for entry in entries.flatten().take(20) {
                        if entry.path().extension().map(|e| e == "swift").unwrap_or(false) {
                            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                                if content.contains("SwiftUI") && !frameworks.contains(&Framework::SwiftUI) {
                                    frameworks.push(Framework::SwiftUI);
                                } else if content.contains("UIKit") && !frameworks.contains(&Framework::UIKit) {
                                    frameworks.push(Framework::UIKit);
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    // Flutter
    if root.join("pubspec.yaml").exists() {
        frameworks.push(Framework::Flutter);
        // Also detect Dart language
    }

    // .NET / C#
    if has_file_with_extension(root, "csproj", 1) || root.join("*.sln").exists() {
        if !frameworks.contains(&Framework::Dotnet) {
            frameworks.push(Framework::Dotnet);
        }
    }

    frameworks
}

// --- Tool/linter detection ---

fn detect_tools(root: &Path) -> Vec<ToolConfig> {
    let mut tools = Vec::new();

    // ESLint
    for name in &[".eslintrc", ".eslintrc.js", ".eslintrc.json", ".eslintrc.yml", "eslint.config.js", "eslint.config.mjs"] {
        if root.join(name).exists() {
            let rules = extract_eslint_rules(root, name);
            tools.push(ToolConfig {
                name: "eslint".into(),
                config_file: name.to_string(),
                rules,
            });
            break;
        }
    }

    // Prettier
    for name in &[".prettierrc", ".prettierrc.json", ".prettierrc.js", ".prettierrc.yml", "prettier.config.js"] {
        if root.join(name).exists() {
            tools.push(ToolConfig {
                name: "prettier".into(),
                config_file: name.to_string(),
                rules: vec!["code-formatting".into()],
            });
            break;
        }
    }

    // Clippy (Rust)
    if root.join("clippy.toml").exists() || root.join(".clippy.toml").exists() {
        tools.push(ToolConfig {
            name: "clippy".into(),
            config_file: "clippy.toml".into(),
            rules: vec!["rust-linting".into()],
        });
    }

    // Rustfmt
    if root.join("rustfmt.toml").exists() || root.join(".rustfmt.toml").exists() {
        tools.push(ToolConfig {
            name: "rustfmt".into(),
            config_file: "rustfmt.toml".into(),
            rules: vec!["rust-formatting".into()],
        });
    }

    // EditorConfig
    if root.join(".editorconfig").exists() {
        tools.push(ToolConfig {
            name: "editorconfig".into(),
            config_file: ".editorconfig".into(),
            rules: vec!["editor-config".into()],
        });
    }

    // TypeScript strict mode
    if root.join("tsconfig.json").exists() {
        if let Ok(content) = std::fs::read_to_string(root.join("tsconfig.json")) {
            if content.contains("\"strict\": true") || content.contains("\"strict\":true") {
                tools.push(ToolConfig {
                    name: "typescript-strict".into(),
                    config_file: "tsconfig.json".into(),
                    rules: vec!["strict-mode".into()],
                });
            }
        }
    }

    // Go: golangci-lint
    for name in &[".golangci.yml", ".golangci.yaml", ".golangci.toml", ".golangci.json"] {
        if root.join(name).exists() {
            tools.push(ToolConfig {
                name: "golangci-lint".into(),
                config_file: name.to_string(),
                rules: vec!["go-linting".into()],
            });
            break;
        }
    }

    // Python: ruff
    for name in &["ruff.toml", ".ruff.toml", "pyproject.toml"] {
        if root.join(name).exists() {
            if *name == "pyproject.toml" {
                if let Ok(content) = std::fs::read_to_string(root.join(name)) {
                    if content.contains("[tool.ruff]") {
                        tools.push(ToolConfig {
                            name: "ruff".into(),
                            config_file: name.to_string(),
                            rules: vec!["python-linting".into(), "python-formatting".into()],
                        });
                        break;
                    }
                }
            } else {
                tools.push(ToolConfig {
                    name: "ruff".into(),
                    config_file: name.to_string(),
                    rules: vec!["python-linting".into(), "python-formatting".into()],
                });
                break;
            }
        }
    }

    // Python: black
    if !tools.iter().any(|t| t.name == "ruff") {
        for name in &["pyproject.toml", ".black.toml"] {
            if root.join(name).exists() {
                if let Ok(content) = std::fs::read_to_string(root.join(name)) {
                    if content.contains("[tool.black]") || *name == ".black.toml" {
                        tools.push(ToolConfig {
                            name: "black".into(),
                            config_file: name.to_string(),
                            rules: vec!["python-formatting".into()],
                        });
                        break;
                    }
                }
            }
        }
    }

    // Python: mypy
    for name in &["mypy.ini", ".mypy.ini", "setup.cfg", "pyproject.toml"] {
        if root.join(name).exists() {
            if *name == "pyproject.toml" || *name == "setup.cfg" {
                if let Ok(content) = std::fs::read_to_string(root.join(name)) {
                    if content.contains("[mypy]") || content.contains("[tool.mypy]") {
                        tools.push(ToolConfig {
                            name: "mypy".into(),
                            config_file: name.to_string(),
                            rules: vec!["python-type-checking".into()],
                        });
                        break;
                    }
                }
            } else {
                tools.push(ToolConfig {
                    name: "mypy".into(),
                    config_file: name.to_string(),
                    rules: vec!["python-type-checking".into()],
                });
                break;
            }
        }
    }

    // Python: flake8
    for name in &[".flake8", "setup.cfg", "tox.ini"] {
        if root.join(name).exists() {
            if *name == ".flake8" {
                tools.push(ToolConfig {
                    name: "flake8".into(),
                    config_file: name.to_string(),
                    rules: vec!["python-linting".into()],
                });
                break;
            }
            if let Ok(content) = std::fs::read_to_string(root.join(name)) {
                if content.contains("[flake8]") {
                    tools.push(ToolConfig {
                        name: "flake8".into(),
                        config_file: name.to_string(),
                        rules: vec!["python-linting".into()],
                    });
                    break;
                }
            }
        }
    }

    // Java: checkstyle
    for name in &["checkstyle.xml", "config/checkstyle/checkstyle.xml"] {
        if root.join(name).exists() {
            tools.push(ToolConfig {
                name: "checkstyle".into(),
                config_file: name.to_string(),
                rules: vec!["java-style".into()],
            });
            break;
        }
    }

    // Java: spotbugs
    if root.join("spotbugs-exclude.xml").exists() || root.join("spotbugs.xml").exists() {
        tools.push(ToolConfig {
            name: "spotbugs".into(),
            config_file: "spotbugs.xml".into(),
            rules: vec!["java-bugs".into()],
        });
    }

    // Swift: swiftlint
    if root.join(".swiftlint.yml").exists() || root.join(".swiftlint.yaml").exists() {
        tools.push(ToolConfig {
            name: "swiftlint".into(),
            config_file: ".swiftlint.yml".into(),
            rules: vec!["swift-linting".into()],
        });
    }

    // Kotlin: ktlint / detekt
    if root.join(".editorconfig").exists() && root.join("build.gradle.kts").exists() {
        // ktlint is often configured via editorconfig + gradle plugin
        if let Ok(content) = std::fs::read_to_string(root.join("build.gradle.kts")) {
            if content.contains("ktlint") {
                tools.push(ToolConfig {
                    name: "ktlint".into(),
                    config_file: "build.gradle.kts".into(),
                    rules: vec!["kotlin-formatting".into()],
                });
            }
        }
    }
    for name in &["detekt.yml", "detekt.yaml", "config/detekt/detekt.yml"] {
        if root.join(name).exists() {
            tools.push(ToolConfig {
                name: "detekt".into(),
                config_file: name.to_string(),
                rules: vec!["kotlin-linting".into()],
            });
            break;
        }
    }

    // Ruby: rubocop
    if root.join(".rubocop.yml").exists() {
        tools.push(ToolConfig {
            name: "rubocop".into(),
            config_file: ".rubocop.yml".into(),
            rules: vec!["ruby-linting".into()],
        });
    }

    // PHP: phpstan / php-cs-fixer
    if root.join("phpstan.neon").exists() || root.join("phpstan.neon.dist").exists() {
        tools.push(ToolConfig {
            name: "phpstan".into(),
            config_file: "phpstan.neon".into(),
            rules: vec!["php-static-analysis".into()],
        });
    }
    if root.join(".php-cs-fixer.php").exists() || root.join(".php-cs-fixer.dist.php").exists() {
        tools.push(ToolConfig {
            name: "php-cs-fixer".into(),
            config_file: ".php-cs-fixer.php".into(),
            rules: vec!["php-formatting".into()],
        });
    }

    // C#: .editorconfig (dotnet-format uses it) + Directory.Build.props
    if root.join("Directory.Build.props").exists() {
        tools.push(ToolConfig {
            name: "dotnet-analyzers".into(),
            config_file: "Directory.Build.props".into(),
            rules: vec!["csharp-analysis".into()],
        });
    }

    tools
}

fn extract_eslint_rules(_root: &Path, _config_file: &str) -> Vec<String> {
    // Simplified: return common rule categories rather than parsing full config
    // Full parsing would require JS/JSON/YAML parser depending on format
    vec!["linting".into()]
}

// --- AI governance detection ---

fn detect_ai_governance(root: &Path) -> AiGovernanceFiles {
    AiGovernanceFiles {
        agents_md: find_file(root, "AGENTS.md"),
        cursorrules: find_file(root, ".cursorrules"),
        windsurfrules: find_file(root, ".windsurfrules"),
        progress_md: find_file_in(root, &[".ai/progress.md", "progress.md"]),
        ai_dir: if root.join(".ai").is_dir() { Some(root.join(".ai")) } else { None },
    }
}

// --- Existing docs detection ---

fn detect_existing_docs(root: &Path) -> ExistingDocs {
    let docs_dir = if root.join("docs").is_dir() {
        Some(root.join("docs"))
    } else {
        None
    };

    let mut doc_files = Vec::new();
    if let Some(ref d) = docs_dir {
        collect_doc_files(d, &mut doc_files, 3);
    }

    ExistingDocs {
        readme: find_file_ci(root, "readme.md"),
        contributing: find_file_ci(root, "contributing.md"),
        changelog: find_file_ci(root, "changelog.md"),
        docs_dir,
        doc_files,
    }
}

// --- Git stats ---

fn detect_git_stats(root: &Path) -> GitStats {
    let git_service = crate::services::git::GitService::open(root);
    let is_git_repo = git_service.is_ok();

    if !is_git_repo {
        return GitStats {
            is_git_repo: false,
            total_commits: 0,
            fix_commits: 0,
            revert_commits: 0,
            recent_fix_patterns: vec![],
        };
    }

    // Use git2 directly for commit log analysis
    let mut total = 0u32;
    let mut fixes = 0u32;
    let mut reverts = 0u32;
    let mut fix_patterns: Vec<String> = Vec::new();

    if let Ok(repo) = git2::Repository::discover(root) {
        if let Ok(mut revwalk) = repo.revwalk() {
            let _ = revwalk.push_head();
            revwalk.set_sorting(git2::Sort::TIME).ok();

            for oid in revwalk.take(100).flatten() {
                if let Ok(commit) = repo.find_commit(oid) {
                    total += 1;
                    let msg = commit.message().unwrap_or("").to_lowercase();
                    if msg.starts_with("fix") || msg.contains("fix:") || msg.contains("bugfix") {
                        fixes += 1;
                        if fix_patterns.len() < 10 {
                            let summary = commit.message().unwrap_or("").lines().next().unwrap_or("").to_string();
                            if summary.len() <= 120 {
                                fix_patterns.push(summary);
                            }
                        }
                    }
                    if msg.starts_with("revert") || msg.contains("revert:") {
                        reverts += 1;
                    }
                }
            }
        }
    }

    GitStats {
        is_git_repo,
        total_commits: total,
        fix_commits: fixes,
        revert_commits: reverts,
        recent_fix_patterns: fix_patterns,
    }
}

// --- Directory structure ---

fn analyze_dir_structure(root: &Path) -> DirStructure {
    let mut top_dirs = Vec::new();
    let mut total_files = 0u32;

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                top_dirs.push(name);
            } else {
                total_files += 1;
            }
        }
    }

    let has_src = top_dirs.iter().any(|d| d == "src" || d == "src-tauri" || d == "lib");
    let has_tests = top_dirs.iter().any(|d| d == "tests" || d == "test" || d == "__tests__" || d == "spec");
    let has_ci = root.join(".github").is_dir()
        || root.join(".gitlab-ci.yml").exists()
        || root.join("Jenkinsfile").exists();

    DirStructure {
        top_level_dirs: top_dirs,
        has_src,
        has_tests,
        has_ci,
        total_files,
        total_lines_code: 0, // Skip expensive counting for now
        total_lines_docs: 0,
    }
}

// --- Dependency detection ---

fn detect_dependencies(root: &Path) -> DependencyInfo {
    let mut npm_deps = Vec::new();
    let mut npm_dev_deps = Vec::new();
    let mut cargo_deps = Vec::new();

    // Parse package.json
    if let Ok(content) = std::fs::read_to_string(root.join("package.json")) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_object()) {
                for (name, ver) in deps {
                    npm_deps.push(DepEntry {
                        name: name.clone(),
                        version: ver.as_str().unwrap_or("*").to_string(),
                    });
                }
            }
            if let Some(deps) = parsed.get("devDependencies").and_then(|v| v.as_object()) {
                for (name, ver) in deps {
                    npm_dev_deps.push(DepEntry {
                        name: name.clone(),
                        version: ver.as_str().unwrap_or("*").to_string(),
                    });
                }
            }
        }
    }

    // Parse Cargo.toml (root or src-tauri)
    let cargo_path = if root.join("src-tauri/Cargo.toml").exists() {
        root.join("src-tauri/Cargo.toml")
    } else {
        root.join("Cargo.toml")
    };
    if let Ok(content) = std::fs::read_to_string(&cargo_path) {
        // Simple TOML parsing: extract [dependencies] section entries
        let mut in_deps = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_deps = trimmed == "[dependencies]" || trimmed == "[dev-dependencies]";
                continue;
            }
            if in_deps {
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                // Parse "name = "version"" or "name = { version = "..." }"
                if let Some(eq_pos) = trimmed.find('=') {
                    let dep_name = trimmed[..eq_pos].trim().to_string();
                    let rest = trimmed[eq_pos + 1..].trim();
                    let version = if rest.starts_with('"') {
                        rest.trim_matches('"').to_string()
                    } else if rest.contains("version") {
                        // { version = "X.Y" ... }
                        rest.split("version")
                            .nth(1)
                            .and_then(|s| s.split('"').nth(1))
                            .unwrap_or("*")
                            .to_string()
                    } else {
                        "*".to_string()
                    };
                    cargo_deps.push(DepEntry {
                        name: dep_name,
                        version,
                    });
                }
            }
        }
    }

    // Parse go.mod
    let mut go_deps = Vec::new();
    if let Ok(content) = std::fs::read_to_string(root.join("go.mod")) {
        let mut in_require = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("require (") || trimmed == "require (" {
                in_require = true;
                continue;
            }
            if in_require && trimmed == ")" {
                in_require = false;
                continue;
            }
            if in_require {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 && !trimmed.starts_with("//") {
                    go_deps.push(DepEntry {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                    });
                }
            }
            // Single-line require
            if trimmed.starts_with("require ") && !trimmed.contains('(') {
                let parts: Vec<&str> = trimmed.strip_prefix("require ").unwrap_or("").split_whitespace().collect();
                if parts.len() >= 2 {
                    go_deps.push(DepEntry {
                        name: parts[0].to_string(),
                        version: parts[1].to_string(),
                    });
                }
            }
        }
    }

    // Parse requirements.txt / pyproject.toml (Python)
    let mut python_deps = Vec::new();
    if let Ok(content) = std::fs::read_to_string(root.join("requirements.txt")) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
                continue;
            }
            // "package==1.0" or "package>=1.0" or "package"
            let (name, ver) = if let Some(pos) = trimmed.find(|c: char| c == '=' || c == '>' || c == '<' || c == '~' || c == '!') {
                (trimmed[..pos].trim().to_string(), trimmed[pos..].trim().to_string())
            } else {
                (trimmed.to_string(), "*".to_string())
            };
            if !name.is_empty() && python_deps.len() < 50 {
                python_deps.push(DepEntry { name, version: ver });
            }
        }
    }
    if python_deps.is_empty() {
        if let Ok(content) = std::fs::read_to_string(root.join("pyproject.toml")) {
            let mut in_deps = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.contains("[project]") || trimmed.contains("dependencies") {
                    if trimmed.contains("dependencies") {
                        in_deps = true;
                        continue;
                    }
                }
                if in_deps && trimmed == "]" {
                    in_deps = false;
                    continue;
                }
                if in_deps {
                    let clean = trimmed.trim_matches(|c: char| c == '"' || c == '\'' || c == ',' || c == ' ');
                    if !clean.is_empty() && !clean.starts_with('[') && python_deps.len() < 50 {
                        python_deps.push(DepEntry { name: clean.to_string(), version: "*".to_string() });
                    }
                }
            }
        }
    }

    // Parse Gemfile (Ruby) — simplified
    let mut ruby_deps = Vec::new();
    if let Ok(content) = std::fs::read_to_string(root.join("Gemfile")) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("gem ") || trimmed.starts_with("gem\t") {
                // gem 'name', '~> 1.0'
                let parts: Vec<&str> = trimmed.split(|c: char| c == '\'' || c == '"').collect();
                if parts.len() >= 2 {
                    let name = parts[1].to_string();
                    let ver = if parts.len() >= 4 { parts[3].to_string() } else { "*".to_string() };
                    ruby_deps.push(DepEntry { name, version: ver });
                }
            }
        }
    }

    // Parse composer.json (PHP)
    let mut php_deps = Vec::new();
    if let Ok(content) = std::fs::read_to_string(root.join("composer.json")) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(deps) = parsed.get("require").and_then(|v| v.as_object()) {
                for (name, ver) in deps {
                    if !name.starts_with("php") && !name.starts_with("ext-") {
                        php_deps.push(DepEntry {
                            name: name.clone(),
                            version: ver.as_str().unwrap_or("*").to_string(),
                        });
                    }
                }
            }
        }
    }

    // Parse pom.xml (Java/Maven) — simplified, extract artifactId
    let mut java_deps = Vec::new();
    if let Ok(content) = std::fs::read_to_string(root.join("pom.xml")) {
        let mut in_dep = false;
        let mut current_artifact = String::new();
        let mut current_version = String::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "<dependency>" { in_dep = true; continue; }
            if trimmed == "</dependency>" {
                if !current_artifact.is_empty() {
                    java_deps.push(DepEntry {
                        name: current_artifact.clone(),
                        version: if current_version.is_empty() { "*".to_string() } else { current_version.clone() },
                    });
                }
                current_artifact.clear();
                current_version.clear();
                in_dep = false;
                continue;
            }
            if in_dep {
                if let (Some(start), Some(end)) = (trimmed.find('>'), trimmed.rfind('<')) {
                    let tag_end = trimmed.find('>').unwrap_or(0);
                    let tag = &trimmed[1..tag_end];
                    let value = &trimmed[tag_end + 1..end];
                    if tag == "artifactId" { current_artifact = value.to_string(); }
                    if tag == "version" { current_version = value.to_string(); }
                    let _ = start; // suppress warning
                }
            }
        }
    }
    // Parse build.gradle (Java/Kotlin) — simplified
    if java_deps.is_empty() {
        for gradle_file in &["build.gradle", "build.gradle.kts", "app/build.gradle", "app/build.gradle.kts"] {
            if let Ok(content) = std::fs::read_to_string(root.join(gradle_file)) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    // implementation 'group:artifact:version' or implementation("group:artifact:version")
                    if (trimmed.starts_with("implementation") || trimmed.starts_with("api") || trimmed.starts_with("compile"))
                        && trimmed.contains(':')
                    {
                        let inner = trimmed.split(|c: char| c == '\'' || c == '"')
                            .find(|s| s.contains(':'))
                            .unwrap_or("");
                        let parts: Vec<&str> = inner.split(':').collect();
                        if parts.len() >= 2 && java_deps.len() < 50 {
                            java_deps.push(DepEntry {
                                name: format!("{}:{}", parts[0], parts[1]),
                                version: parts.get(2).unwrap_or(&"*").to_string(),
                            });
                        }
                    }
                }
                break;
            }
        }
    }

    // Parse Package.swift (Swift/iOS) — simplified
    let mut swift_deps = Vec::new();
    if let Ok(content) = std::fs::read_to_string(root.join("Package.swift")) {
        for line in content.lines() {
            let trimmed = line.trim();
            // .package(url: "https://github.com/...", from: "1.0.0")
            if trimmed.contains(".package(") && trimmed.contains("url:") {
                if let Some(url_start) = trimmed.find("url:") {
                    let rest = &trimmed[url_start + 4..];
                    let url = rest.split('"').nth(1).unwrap_or("");
                    let name = url.rsplit('/').next().unwrap_or(url)
                        .trim_end_matches(".git").to_string();
                    let ver = rest.split('"').nth(3).unwrap_or("*").to_string();
                    if !name.is_empty() {
                        swift_deps.push(DepEntry { name, version: ver });
                    }
                }
            }
        }
    }

    DependencyInfo {
        npm_deps, npm_dev_deps, cargo_deps,
        go_deps, python_deps, java_deps,
        swift_deps, ruby_deps, php_deps,
    }
}

// --- CI config detection ---

fn detect_ci_config(root: &Path) -> Option<CiConfig> {
    // GitHub Actions
    let gh_dir = root.join(".github/workflows");
    if gh_dir.is_dir() {
        let mut workflows = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&gh_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "yml" || e == "yaml").unwrap_or(false) {
                    let name = path.file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let (triggers, steps) = parse_github_workflow(&path);
                    workflows.push(CiWorkflow {
                        name,
                        triggers,
                        steps_summary: steps,
                    });
                }
            }
        }
        if !workflows.is_empty() {
            return Some(CiConfig {
                provider: "GitHub Actions".into(),
                workflows,
            });
        }
    }

    // GitLab CI
    if root.join(".gitlab-ci.yml").exists() {
        return Some(CiConfig {
            provider: "GitLab CI".into(),
            workflows: vec![CiWorkflow {
                name: "gitlab-ci".into(),
                triggers: vec!["push".into()],
                steps_summary: vec!["(see .gitlab-ci.yml)".into()],
            }],
        });
    }

    None
}

fn parse_github_workflow(path: &Path) -> (Vec<String>, Vec<String>) {
    let mut triggers = Vec::new();
    let mut steps = Vec::new();

    if let Ok(content) = std::fs::read_to_string(path) {
        let mut in_on = false;
        let mut in_steps = false;
        for line in content.lines() {
            let trimmed = line.trim();
            // Detect triggers (on: section)
            if trimmed == "on:" || trimmed.starts_with("on:") {
                in_on = true;
                in_steps = false;
                if trimmed.len() > 3 {
                    // inline: "on: [push, pull_request]"
                    let rest = trimmed[3..].trim();
                    for t in rest.trim_matches(|c| c == '[' || c == ']').split(',') {
                        let t = t.trim().to_string();
                        if !t.is_empty() {
                            triggers.push(t);
                        }
                    }
                    in_on = false;
                }
                continue;
            }
            if in_on && trimmed.ends_with(':') && !trimmed.starts_with('-') && !trimmed.starts_with('#') {
                triggers.push(trimmed.trim_end_matches(':').to_string());
            }
            if in_on && !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.is_empty() {
                in_on = false;
            }

            // Detect steps (- name: or - run:)
            if trimmed.starts_with("- name:") {
                in_steps = true;
                let name = trimmed.strip_prefix("- name:").unwrap_or("").trim().to_string();
                if !name.is_empty() && steps.len() < 15 {
                    steps.push(name);
                }
            }
            if trimmed.starts_with("- run:") && !in_steps {
                let cmd = trimmed.strip_prefix("- run:").unwrap_or("").trim().to_string();
                if !cmd.is_empty() && steps.len() < 15 {
                    steps.push(cmd);
                }
            }
            if trimmed.starts_with("- name:") || trimmed.starts_with("- uses:") {
                in_steps = false;
            }
        }
    }

    (triggers, steps)
}

// --- Helpers ---

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    let p = root.join(name);
    if p.exists() { Some(p) } else { None }
}

fn find_file_in(root: &Path, candidates: &[&str]) -> Option<PathBuf> {
    for c in candidates {
        let p = root.join(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn find_file_ci(root: &Path, name_lower: &str) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_lowercase();
            if fname == name_lower {
                return Some(entry.path());
            }
        }
    }
    None
}

fn has_file_with_extension(root: &Path, ext: &str, max_depth: u32) -> bool {
    has_file_with_extension_inner(root, ext, 0, max_depth)
}

fn has_file_with_extension_inner(dir: &Path, ext: &str, depth: u32, max_depth: u32) -> bool {
    if depth > max_depth {
        return false;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(e) = path.extension() {
                    if e.to_string_lossy() == ext {
                        return true;
                    }
                }
            } else if path.is_dir() && depth < max_depth {
                if has_file_with_extension_inner(&path, ext, depth + 1, max_depth) {
                    return true;
                }
            }
        }
    }
    false
}

fn collect_doc_files(dir: &Path, out: &mut Vec<PathBuf>, max_depth: u32) {
    collect_doc_files_inner(dir, out, 0, max_depth);
}

fn collect_doc_files_inner(dir: &Path, out: &mut Vec<PathBuf>, depth: u32, max_depth: u32) {
    if depth > max_depth || out.len() >= 50 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "md" || ext_str == "txt" || ext_str == "rst" {
                        out.push(path);
                    }
                }
            } else if path.is_dir() {
                collect_doc_files_inner(&path, out, depth + 1, max_depth);
            }
        }
    }
}

fn read_package_json_deps(root: &Path) -> Option<HashMap<String, serde_json::Value>> {
    let content = std::fs::read_to_string(root.join("package.json")).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    let mut all_deps = HashMap::new();

    if let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_object()) {
        for (k, v) in deps {
            all_deps.insert(k.clone(), v.clone());
        }
    }
    if let Some(deps) = parsed.get("devDependencies").and_then(|v| v.as_object()) {
        for (k, v) in deps {
            all_deps.insert(k.clone(), v.clone());
        }
    }

    Some(all_deps)
}
