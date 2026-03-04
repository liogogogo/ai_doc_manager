import { useState, useEffect, useCallback, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTauriCommand } from "@/hooks/useTauriCommand";
import { useProjectStore } from "@/stores/projectStore";
import { cn } from "@/lib/utils";
import {
  Check,
  CheckCircle2,
  FileCode2,
  Loader2,
  Trash2,
  Sparkles,
  ShieldCheck,
  ShieldAlert,
  X,
  PartyPopper,
  Bot,
  Settings2,
  Send,
  Zap,
  FolderOpen,
  AlertTriangle,
  Plus,
  Shield,
  Activity,
  RefreshCw,
  StopCircle,
  Brain,
  ChevronDown,
  ChevronUp,
} from "lucide-react";

// --- Types matching Rust backend ---

interface TechScanResult {
  root_path: string;
  project_name: string;
  languages: string[];
  frameworks: string[];
  tools: { name: string; config_file: string; rules: string[] }[];
  ai_governance: {
    agents_md: string | null;
    cursorrules: string | null;
    windsurfrules: string | null;
    progress_md: string | null;
    ai_dir: string | null;
  };
  existing_docs: {
    readme: string | null;
    contributing: string | null;
    changelog: string | null;
    docs_dir: string | null;
    doc_files: string[];
  };
  git_stats: {
    is_git_repo: boolean;
    total_commits: number;
    fix_commits: number;
    revert_commits: number;
    recent_fix_patterns: string[];
  };
  dir_structure: {
    top_level_dirs: string[];
    has_src: boolean;
    has_tests: boolean;
    has_ci: boolean;
    total_files: number;
  };
  dependencies: {
    npm_deps: { name: string; version: string }[];
    npm_dev_deps: { name: string; version: string }[];
    cargo_deps: { name: string; version: string }[];
    go_deps: { name: string; version: string }[];
    python_deps: { name: string; version: string }[];
    java_deps: { name: string; version: string }[];
    swift_deps: { name: string; version: string }[];
    ruby_deps: { name: string; version: string }[];
    php_deps: { name: string; version: string }[];
  };
  ci_config: {
    provider: string;
    workflows: { name: string; triggers: string[]; steps_summary: string[] }[];
  } | null;
}

interface GeneratedRule {
  id: string;
  category: string;
  content: string;
  source: Record<string, unknown>;
  accepted: boolean;
}

interface GeneratedFile {
  rel_path: string;
  content: string;
  description: string;
  overwrite: boolean;
}

interface InitPlan {
  scan_result: TechScanResult;
  mode: string;
  rules: GeneratedRule[];
  files: GeneratedFile[];
}

interface BackendProject {
  id: string;
  name: string;
  root_path: string;
  config: Record<string, unknown>;
  created_at: number;
  updated_at: number;
}

interface GovernanceStatus {
  has_agents_md: boolean;
  has_progress_md: boolean;
  has_config_toml: boolean;
  agents_md_path: string | null;
}

interface DriftItem {
  category: string;
  description: string;
  severity: string;
}

interface GovernanceDrift {
  is_stale: boolean;
  drifts: DriftItem[];
  current_version: string;
  agents_md_len: number;
}

interface GovernanceFileContent {
  name: string;
  path: string;
  content: string;
  exists: boolean;
}

interface ViolationItem {
  id: string;
  project_id: string;
  category: string;
  severity: string;
  file_path: string;
  line_number: number | null;
  description: string;
  rule_ref: string;
  status: string;
  detected_at: number;
}

interface ComplianceReport {
  project_id: string;
  total: number;
  high: number;
  medium: number;
  low: number;
  violations: ViolationItem[];
  checked_at: number;
}

// --- Main Page ---

export function ProjectsPage() {
  const { projects, currentProject, setCurrentProject, setProjects, removeProject: removeFromStore } = useProjectStore();

  const listCmd = useTauriCommand<BackendProject[]>("list_projects");
  const addCmd = useTauriCommand<BackendProject>("add_project");
  const removeCmd = useTauriCommand<void>("remove_project");
  const scanCmd = useTauriCommand<InitPlan>("scan_project");
  const confirmCmd = useTauriCommand<string[]>("confirm_init");
  const govCmd = useTauriCommand<GovernanceStatus>("check_governance");
  const driftCmd = useTauriCommand<GovernanceDrift>("check_governance_freshness");
  const suggestCmd = useTauriCommand<string>("suggest_governance_updates");
  const applyCmd = useTauriCommand<string>("apply_governance_updates");
  const readFileCmd = useTauriCommand<GovernanceFileContent>("read_governance_file");

  // Governance status per project (keyed by project id)
  const [govMap, setGovMap] = useState<Record<string, GovernanceStatus>>({});
  const [driftMap, setDriftMap] = useState<Record<string, GovernanceDrift>>({});
  const [showDriftPanel, setShowDriftPanel] = useState<string | null>(null); // project id
  const [driftSuggestions, setDriftSuggestions] = useState<string | null>(null);
  const [isSuggesting, setIsSuggesting] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [applySuccess, setApplySuccess] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState<{ id: string; name: string } | null>(null);
  const [regeneratingId, setRegeneratingId] = useState<string | null>(null);
  // driftContext holds pre-built drift summary to pass into the wizard when regenerating from drift panel
  const [regenDriftContext, setRegenDriftContext] = useState<string | null>(null);
  const [confirmRegen, setConfirmRegen] = useState<{ id: string; name: string; rootPath: string; driftContext?: string } | null>(null);

  // Compliance check state
  const complianceCmd = useTauriCommand<ComplianceReport>("run_compliance_check");
  const dismissCmd = useTauriCommand<void>("update_violation_status");
  const gitHooksCmd = useTauriCommand<string>("setup_git_hooks");
  const [complianceMap, setComplianceMap] = useState<Record<string, ComplianceReport>>({});
  const [showCompliancePanel, setShowCompliancePanel] = useState<string | null>(null);
  const [isCheckingCompliance, setIsCheckingCompliance] = useState<string | null>(null);
  const [hookSetupResult, setHookSetupResult] = useState<Record<string, string>>({});

  // File viewer modal state
  const [fileViewer, setFileViewer] = useState<{ projectName: string; rootPath: string; fileType: string; fileName: string } | null>(null);
  const [fileContent, setFileContent] = useState<GovernanceFileContent | null>(null);
  const [isLoadingFile, setIsLoadingFile] = useState(false);

  // Filter state for stats cards
  const [filter, setFilter] = useState<'all' | 'governed' | 'needs_update'>('all');

  // Init wizard state
  const [showWizard, setShowWizard] = useState(false);
  const [showConfirmDialog, setShowConfirmDialog] = useState(false);
  const [wizardPlan, setWizardPlan] = useState<InitPlan | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const [autoGenerate, setAutoGenerate] = useState(false);
  // Which project is being scanned for init (keyed by project id)
  const [scanningInit, setScanningInit] = useState<string | null>(null);

  // Success/info feedback
  type FeedbackInfo = {
    type: "success" | "info";
    title: string;
    detail: string;
    files?: string[];
  };
  const [feedback, setFeedback] = useState<FeedbackInfo | null>(null);
  const [initSuccess, setInitSuccess] = useState<string[] | null>(null); // files written in wizard

  // Load projects on mount
  useEffect(() => {
    loadProjects();
  }, []);

  // Auto-reset filter when projects are empty
  useEffect(() => {
    if (projects.length === 0 && filter !== 'all') {
      setFilter('all');
    }
  }, [projects.length, filter]);

  const loadProjects = useCallback(async () => {
    const result = await listCmd.execute();
    if (result) {
      setProjects(
        result.map((p) => ({
          id: p.id,
          name: p.name,
          rootPath: p.root_path,
          healthScore: 0,
          docCount: 0,
          conflictCount: 0,
          staleCount: 0,
          lastGcAt: null,
        }))
      );
      // Check governance status + freshness for each project
      const newGovMap: Record<string, GovernanceStatus> = {};
      const newDriftMap: Record<string, GovernanceDrift> = {};
      for (const p of result) {
        const status = await govCmd.execute({ root_path: p.root_path });
        if (status) {
          newGovMap[p.id] = status;
          // Check freshness for projects with AGENTS.md
          if (status.has_agents_md) {
            const drift = await driftCmd.execute({ root_path: p.root_path });
            if (drift) {
              newDriftMap[p.id] = drift;
            }
          }
        }
      }
      setGovMap(newGovMap);
      setDriftMap(newDriftMap);
    }
  }, []);

  const [addError, setAddError] = useState<string | null>(null);

  const handleAddProject = async () => {
    setAddError(null);
    setFilter('all'); // Ensure we see the new project

    // Check if running in Tauri environment
    // @ts-ignore
    if (typeof window !== 'undefined' && !window.__TAURI_INTERNALS__) {
      setAddError("环境错误：无法调用系统 API。请确保您正在使用 DocGuardian 桌面客户端，而不是在浏览器中预览。");
      return;
    }

    try {
      // Open native folder picker
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择项目文件夹",
      });
      if (!selected) return; // user cancelled

      const folderPath = typeof selected === "string" ? selected : selected[0];
      if (!folderPath) return;

      console.log("[handleAddProject] 选择的路径:", folderPath);
      setIsAdding(true);

      // Extract name from path
      const name = folderPath.split("/").filter(Boolean).pop() || "project";

      // Add to DB (backend canonicalizes path)
      console.log("[handleAddProject] 调用 add_project, name:", name, "root_path:", folderPath);
      const project = await addCmd.execute({ name, root_path: folderPath });
      console.log("[handleAddProject] add_project 结果:", project, "错误:", addCmd.error);
      if (!project) {
        setAddError(addCmd.error || "添加项目失败");
        setIsAdding(false);
        return;
      }

      // Scan for AI governance
      const plan = await scanCmd.execute({ root_path: project.root_path });
      setIsAdding(false);

      if (plan) {
        const hasGovernance = plan.scan_result.ai_governance.agents_md !== null;
        if (!hasGovernance) {
          setWizardPlan(plan);
          setShowConfirmDialog(true);
        } else {
          // Already has governance → show info feedback
          setFeedback({
            type: "info",
            title: `${plan.scan_result.project_name} 已有 AI 治理框架`,
            detail: "检测到 AGENTS.md，项目已添加到列表。",
          });
          setTimeout(() => setFeedback(null), 6000);
        }
      } else {
        // Scan failed but project added
        setFeedback({
          type: "info",
          title: "项目已添加",
          detail: "治理框架检测未完成，你可以稍后手动初始化。",
        });
        setTimeout(() => setFeedback(null), 6000);
      }

      await loadProjects();
    } catch (err) {
      console.error("Failed to add project:", err);
      setAddError("操作失败：" + String(err));
      setIsAdding(false);
    }
  };

  const handleRemoveProject = async (id: string) => {
    await removeCmd.execute({ project_id: id });
    removeFromStore(id);
  };

  /** Called when user confirms regeneration of an existing governance framework.
   *  @param driftContext  Optional pre-built drift summary to pre-fill the refinement input after generation.
   */
  const handleRegenerateGovernance = async (projectId: string, rootPath: string, driftContext?: string) => {
    setRegeneratingId(projectId);
    setRegenDriftContext(driftContext ?? null);
    const plan = await scanCmd.execute({ root_path: rootPath });
    setRegeneratingId(null);
    if (plan) {
      // Force overwrite so confirm_init will replace the existing AGENTS.md
      const updatedPlan = {
        ...plan,
        files: plan.files.map((f) =>
          f.rel_path === "AGENTS.md" ? { ...f, overwrite: true } : f
        ),
      };
      setWizardPlan(updatedPlan);
      setAutoGenerate(true);
      setShowWizard(true);
    }
  };

  /** Called when user clicks "未初始化" badge on an existing project */
  const handleInitExistingProject = async (
    e: React.MouseEvent,
    projectId: string,
    rootPath: string,
  ) => {
    e.stopPropagation();
    setScanningInit(projectId);
    const plan = await scanCmd.execute({ root_path: rootPath });
    setScanningInit(null);
    if (plan) {
      setWizardPlan(plan);
      setShowConfirmDialog(true);
    }
  };

  const handleConfirmInit = async () => {
    if (!wizardPlan) return;
    const written = await confirmCmd.execute({
      root_path: wizardPlan.scan_result.root_path,
      plan: wizardPlan,
    });
    if (written) {
      // Show success state inside modal
      setInitSuccess(written);
    }
  };

  const handleCloseWizard = () => {
    if (initSuccess) {
      setFeedback({
        type: "success",
        title: "AI 治理框架已生成",
        detail: `已写入 ${initSuccess.length} 个文件到项目中。`,
        files: initSuccess,
      });
      setTimeout(() => setFeedback(null), 10000);
    }
    setShowWizard(false);
    setWizardPlan(null);
    setInitSuccess(null);
    loadProjects();
  };

  const handleUpdateFileContent = (fileIdx: number, content: string) => {
    if (!wizardPlan) return;
    setWizardPlan({
      ...wizardPlan,
      files: wizardPlan.files.map((f, i) =>
        i === fileIdx ? { ...f, content } : f
      ),
    });
  };

  const handleOpenFileViewer = async (rootPath: string, fileType: string, fileName: string, projectName: string) => {
    setFileViewer({ projectName, rootPath, fileType, fileName });
    setIsLoadingFile(true);
    setFileContent(null);
    const result = await readFileCmd.execute({ root_path: rootPath, file_type: fileType });
    setFileContent(result);
    setIsLoadingFile(false);
  };

  const handleCloseFileViewer = () => {
    setFileViewer(null);
    setFileContent(null);
  };

  const handleRunComplianceCheck = async (projectId: string, rootPath: string) => {
    setIsCheckingCompliance(projectId);
    try {
      const result = await complianceCmd.execute({ project_id: projectId, root_path: rootPath });
      if (result) {
        setComplianceMap(prev => ({ ...prev, [projectId]: result }));
        setShowCompliancePanel(projectId);
      }
    } finally {
      setIsCheckingCompliance(null);
    }
  };

  const handleDismissViolation = async (violationId: string, projectId: string) => {
    await dismissCmd.execute({ violation_id: violationId, new_status: "dismissed" });
    setComplianceMap(prev => {
      const report = prev[projectId];
      if (!report) return prev;
      const updated = report.violations.map(v =>
        v.id === violationId ? { ...v, status: "dismissed" } : v
      );
      const open = updated.filter(v => v.status === "open");
      return {
        ...prev,
        [projectId]: {
          ...report,
          violations: updated,
          total: open.length,
          high: open.filter(v => v.severity === "high").length,
          medium: open.filter(v => v.severity === "medium").length,
          low: open.filter(v => v.severity === "low").length,
        },
      };
    });
  };

  // Computed stats for the header
  const totalProjects = projects.length;
  const governedCount = Object.values(govMap).filter(g => g.has_agents_md).length;
  const driftCount = Object.values(driftMap).filter(d => d.is_stale).length;

  // Filter projects based on selection
  const filteredProjects = projects.filter((project) => {
    const gov = govMap[project.id];
    const drift = driftMap[project.id];
    
    if (filter === 'governed') {
      return gov?.has_agents_md;
    }
    if (filter === 'needs_update') {
      return drift?.is_stale;
    }
    return true;
  });

  return (
    <div className="space-y-5">
      {/* ── Page header with stats ── */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-bold text-gray-900 tracking-tight">项目管理</h1>
          <p className="mt-1 text-sm text-gray-500">管理 AI 编码治理框架，让每个项目都有章可循</p>
        </div>
        <button
          onClick={handleAddProject}
          disabled={isAdding}
          className="inline-flex items-center gap-2 rounded-xl bg-gray-900 px-4 py-2.5 text-sm font-medium text-white shadow-sm transition-all hover:bg-gray-800 active:scale-[0.98] disabled:opacity-50"
        >
          {isAdding ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plus className="h-4 w-4" />}
          添加项目
        </button>
      </div>

      {/* ── Stats overview bar ── */}
      {totalProjects > 0 && (
        <div className="grid grid-cols-3 gap-3">
          <div 
            onClick={() => setFilter('all')}
            className={cn(
              "rounded-xl bg-white border px-4 py-3 shadow-sm cursor-pointer transition-all hover:shadow-md",
              filter === 'all' 
                ? "border-brand-500 ring-1 ring-brand-500" 
                : "border-gray-100 hover:border-gray-200"
            )}
          >
            <div className="flex items-center gap-2">
              <div className="rounded-lg bg-blue-50 p-1.5">
                <FolderOpen className="h-3.5 w-3.5 text-blue-600" />
              </div>
              <span className="text-xs font-medium text-gray-500">项目总数</span>
            </div>
            <p className="mt-1.5 text-2xl font-bold tabular-nums text-gray-900">{totalProjects}</p>
          </div>
          
          <div 
            onClick={() => setFilter('governed')}
            className={cn(
              "rounded-xl bg-white border px-4 py-3 shadow-sm cursor-pointer transition-all hover:shadow-md",
              filter === 'governed'
                ? "border-green-500 ring-1 ring-green-500"
                : "border-gray-100 hover:border-gray-200"
            )}
          >
            <div className="flex items-center gap-2">
              <div className="rounded-lg bg-green-50 p-1.5">
                <ShieldCheck className="h-3.5 w-3.5 text-green-600" />
              </div>
              <span className="text-xs font-medium text-gray-500">已治理</span>
            </div>
            <p className="mt-1.5 text-2xl font-bold tabular-nums text-gray-900">
              {governedCount}
              <span className="ml-1.5 text-xs font-normal text-gray-400">/ {totalProjects}</span>
            </p>
          </div>

          <div 
            onClick={() => setFilter('needs_update')}
            className={cn(
              "rounded-xl bg-white border px-4 py-3 shadow-sm cursor-pointer transition-all hover:shadow-md",
              filter === 'needs_update'
                ? "border-orange-500 ring-1 ring-orange-500"
                : "border-gray-100 hover:border-gray-200"
            )}
          >
            <div className="flex items-center gap-2">
              <div className={cn("rounded-lg p-1.5", driftCount > 0 ? "bg-orange-50" : "bg-gray-50")}>
                <Activity className={cn("h-3.5 w-3.5", driftCount > 0 ? "text-orange-600" : "text-gray-400")} />
              </div>
              <span className="text-xs font-medium text-gray-500">需更新</span>
            </div>
            <p className={cn("mt-1.5 text-2xl font-bold tabular-nums", driftCount > 0 ? "text-orange-600" : "text-gray-900")}>
              {driftCount}
            </p>
          </div>
        </div>
      )}

      {/* ── Feedback banner ── */}
      {feedback && (
        <div className={cn(
          "rounded-xl flex items-start gap-3 p-4 border shadow-sm",
          feedback.type === "success"
            ? "border-green-200/60 bg-green-50/80"
            : "border-blue-200/60 bg-blue-50/80"
        )}>
          {feedback.type === "success" ? (
            <CheckCircle2 className="mt-0.5 h-5 w-5 shrink-0 text-green-600" />
          ) : (
            <ShieldCheck className="mt-0.5 h-5 w-5 shrink-0 text-blue-600" />
          )}
          <div className="flex-1 min-w-0">
            <p className={cn(
              "text-sm font-medium",
              feedback.type === "success" ? "text-green-800" : "text-blue-800"
            )}>
              {feedback.title}
            </p>
            <p className={cn(
              "text-xs mt-0.5",
              feedback.type === "success" ? "text-green-600" : "text-blue-600"
            )}>
              {feedback.detail}
            </p>
            {feedback.files && feedback.files.length > 0 && (
              <div className="mt-2 flex flex-wrap gap-1.5">
                {feedback.files.map((f) => (
                  <span key={f} className="inline-flex items-center gap-1 rounded-md bg-green-100/80 px-2 py-0.5 text-xs font-mono text-green-700">
                    <FileCode2 className="h-3 w-3" />
                    {f}
                  </span>
                ))}
              </div>
            )}
          </div>
          <button
            onClick={() => setFeedback(null)}
            className={cn(
              "shrink-0 rounded-lg p-1 transition-colors",
              feedback.type === "success" ? "hover:bg-green-100 text-green-400" : "hover:bg-blue-100 text-blue-400"
            )}
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      )}

      {/* Adding state */}
      {isAdding && (
        <div className="rounded-xl bg-white border border-gray-100 flex items-center gap-3 p-4 shadow-sm">
          <Loader2 className="h-5 w-5 animate-spin text-brand-500" />
          <span className="text-sm text-gray-600">正在扫描项目技术栈，请稍候…</span>
        </div>
      )}
      {addError && (
        <div className="rounded-xl border border-red-200 bg-red-50 p-4 shadow-sm">
          <p className="text-sm text-red-600">{addError}</p>
        </div>
      )}

      {/* ── Project list ── */}
      {projects.length === 0 && !isAdding ? (
        <EmptyState onAdd={handleAddProject} />
      ) : (
        <div className="space-y-3">
          {filteredProjects.length === 0 && !isAdding ? (
            <div className="text-center py-12 text-gray-400">
               <div className="mx-auto w-fit rounded-full bg-gray-50 p-4 mb-3">
                  <FolderOpen className="h-8 w-8 text-gray-300" />
               </div>
               <p className="text-sm">在此筛选条件下未找到项目</p>
            </div>
          ) : (
            filteredProjects.map((project) => {
              const gov = govMap[project.id];
              const drift = driftMap[project.id];
              const isSelected = currentProject?.id === project.id;
              const hasAgents = gov?.has_agents_md;
              const isStale = drift?.is_stale;
              const isDriftOpen = showDriftPanel === project.id;

            // Governance file indicators
            const govFiles = gov ? [
              { name: "AGENTS.md", exists: gov.has_agents_md, fileType: "agents_md" },
              { name: "progress.md", exists: gov.has_progress_md, fileType: "progress_md" },
              { name: ".toml", exists: gov.has_config_toml, fileType: "config_toml" },
            ] : [];

            return (
              <div key={project.id} className="group">
                <div
                  onClick={() => setCurrentProject(project)}
                  className={cn(
                    "relative rounded-xl bg-white border shadow-sm cursor-pointer transition-all",
                    isSelected
                      ? "border-brand-300 ring-2 ring-brand-500/20 shadow-md"
                      : "border-gray-150 hover:border-gray-250 hover:shadow-md",
                    isDriftOpen && "rounded-b-none border-b-0"
                  )}
                >
                  <div className="px-5 py-4">
                    {/* Row 1: Name + governance badge + actions */}
                    <div className="flex items-center gap-3">
                      {/* Project icon */}
                      <div className={cn(
                        "shrink-0 rounded-lg p-2 transition-colors",
                        isSelected ? "bg-brand-50" : "bg-gray-50 group-hover:bg-gray-100"
                      )}>
                        <FolderOpen className={cn(
                          "h-4.5 w-4.5",
                          isSelected ? "text-brand-600" : "text-gray-500"
                        )} />
                      </div>

                      {/* Project name + path */}
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <h3 className="text-sm font-semibold text-gray-900 truncate">
                            {project.name}
                          </h3>
                          {isSelected && (
                            <span className="inline-flex items-center rounded-md bg-brand-50 px-1.5 py-0.5 text-[10px] font-semibold text-brand-700 uppercase tracking-wide">
                              当前
                            </span>
                          )}
                        </div>
                        <p className="mt-0.5 text-xs text-gray-400 truncate font-mono">
                          {project.rootPath}
                        </p>
                      </div>

                      {/* Governance status badge */}
                      <div className="flex items-center gap-2 shrink-0">
                        {gov ? (
                          !hasAgents ? (
                            <button
                              onClick={(e) => handleInitExistingProject(e, project.id, project.rootPath)}
                              disabled={scanningInit === project.id}
                              className="inline-flex items-center gap-1.5 rounded-lg bg-amber-50 border border-amber-200/60 px-2.5 py-1 text-xs font-medium text-amber-700 hover:bg-amber-100 hover:border-amber-300 transition-colors disabled:opacity-60"
                              title="点击立即初始化 AI 治理框架"
                            >
                              {scanningInit === project.id ? (
                                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                              ) : (
                                <AlertTriangle className="h-3.5 w-3.5" />
                              )}
                              {scanningInit === project.id ? "扫描中…" : "未初始化"}
                            </button>
                          ) : isStale ? (
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                setShowDriftPanel(isDriftOpen ? null : project.id);
                                setDriftSuggestions(null);
                                setApplySuccess(false);
                              }}
                              className={cn(
                                "inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1 text-xs font-medium transition-colors",
                                isDriftOpen
                                  ? "bg-orange-100 border-orange-300 text-orange-800"
                                  : "bg-orange-50 border-orange-200/60 text-orange-700 hover:bg-orange-100"
                              )}
                            >
                              <ShieldAlert className="h-3.5 w-3.5" />
                              {drift.drifts.length} 项偏差
                            </button>
                          ) : (
                            <span className="inline-flex items-center gap-1.5 rounded-lg bg-green-50 border border-green-200/60 px-2.5 py-1 text-xs font-medium text-green-700">
                              <ShieldCheck className="h-3.5 w-3.5" />
                              v{drift?.current_version || "?"}
                            </span>
                          )
                        ) : null}

                        {/* Regenerate button — visible on hover for governed projects */}
                        {hasAgents && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              setConfirmRegen({ id: project.id, name: project.name, rootPath: project.rootPath });
                            }}
                            disabled={regeneratingId === project.id}
                            className="opacity-0 group-hover:opacity-100 inline-flex items-center gap-1 rounded-lg bg-gray-100 px-2 py-1 text-[11px] font-medium text-gray-500 hover:bg-violet-50 hover:text-violet-600 border border-transparent hover:border-violet-200/60 transition-all disabled:opacity-40"
                            title="重新生成 AGENTS.md"
                          >
                            {regeneratingId === project.id ? (
                              <Loader2 className="h-3 w-3 animate-spin" />
                            ) : (
                              <RefreshCw className="h-3 w-3" />
                            )}
                            重新生成
                          </button>
                        )}

                        {/* Compliance check button */}
                        {hasAgents && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              if (showCompliancePanel === project.id) {
                                setShowCompliancePanel(null);
                              } else {
                                handleRunComplianceCheck(project.id, project.rootPath);
                              }
                            }}
                            disabled={isCheckingCompliance === project.id}
                            className={cn(
                              "inline-flex items-center gap-1 rounded-lg px-2 py-1 text-[11px] font-medium border transition-all",
                              showCompliancePanel === project.id
                                ? "bg-blue-100 border-blue-300 text-blue-800"
                                : complianceMap[project.id] && complianceMap[project.id].total > 0
                                  ? "bg-red-50 border-red-200/60 text-red-700 hover:bg-red-100"
                                  : "opacity-0 group-hover:opacity-100 bg-gray-100 border-transparent text-gray-500 hover:bg-blue-50 hover:text-blue-600 hover:border-blue-200/60",
                              isCheckingCompliance === project.id && "opacity-100"
                            )}
                            title="运行合规检查"
                          >
                            {isCheckingCompliance === project.id ? (
                              <Loader2 className="h-3 w-3 animate-spin" />
                            ) : (
                              <Activity className="h-3 w-3" />
                            )}
                            {complianceMap[project.id] && complianceMap[project.id].total > 0
                              ? `${complianceMap[project.id].total} 违规`
                              : "合规检查"}
                          </button>
                        )}
                      </div>
                    </div>

                    {/* Row 2: Governance file pills + scan info + remove */}
                    {gov && (
                      <div className="mt-3 flex items-center gap-4">
                        {/* Gov file indicators */}
                        <div className="flex items-center gap-1.5">
                          {govFiles.map((f) => (
                            <button
                              key={f.name}
                              onClick={(e) => {
                                e.stopPropagation();
                                if (f.exists) {
                                  handleOpenFileViewer(project.rootPath, f.fileType, f.name, project.name);
                                }
                              }}
                              disabled={!f.exists}
                              className={cn(
                                "inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[11px] font-mono transition-colors",
                                f.exists
                                  ? "bg-green-50 text-green-700 hover:bg-green-100 cursor-pointer"
                                  : "bg-gray-50 text-gray-400 cursor-not-allowed"
                              )}
                              title={f.exists ? "点击查看内容" : "文件不存在"}
                            >
                              {f.exists ? (
                                <Check className="h-2.5 w-2.5" />
                              ) : (
                                <X className="h-2.5 w-2.5" />
                              )}
                              {f.name}
                            </button>
                          ))}
                        </div>

                        {/* Version + length info */}
                        {drift && !isStale && (
                          <span className="text-[11px] text-gray-400">
                            {drift.agents_md_len.toLocaleString()} 字符
                          </span>
                        )}

                        {/* Severity summary when stale */}
                        {drift && isStale && (() => {
                          const h = drift.drifts.filter(d => d.severity === "high").length;
                          const m = drift.drifts.filter(d => d.severity === "medium").length;
                          const l = drift.drifts.filter(d => d.severity === "low").length;
                          return (
                            <div className="flex items-center gap-2">
                              {h > 0 && <span className="inline-flex items-center gap-0.5 text-[11px] text-red-600"><span className="inline-block w-1.5 h-1.5 rounded-full bg-red-500" />{h} 高</span>}
                              {m > 0 && <span className="inline-flex items-center gap-0.5 text-[11px] text-orange-600"><span className="inline-block w-1.5 h-1.5 rounded-full bg-orange-400" />{m} 中</span>}
                              {l > 0 && <span className="inline-flex items-center gap-0.5 text-[11px] text-gray-500"><span className="inline-block w-1.5 h-1.5 rounded-full bg-gray-300" />{l} 低</span>}
                            </div>
                          );
                        })()}

                        {/* Remove button — far right, separated from badges */}
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            setConfirmRemove({ id: project.id, name: project.name });
                          }}
                          className="ml-auto shrink-0 rounded-lg p-1.5 text-gray-300 opacity-0 group-hover:opacity-100 hover:bg-red-50 hover:text-red-500 transition-all"
                          title="移除项目"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    )}
                    {/* Remove button fallback when no gov data */}
                    {!gov && (
                      <div className="mt-3 flex items-center">
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            setConfirmRemove({ id: project.id, name: project.name });
                          }}
                          className="ml-auto shrink-0 rounded-lg p-1.5 text-gray-300 opacity-0 group-hover:opacity-100 hover:bg-red-50 hover:text-red-500 transition-all"
                          title="移除项目"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    )}
                  </div>
                </div>

                {/* ── Inline drift detail panel ── */}
                {isDriftOpen && drift && (() => {
                  return (
                    <div className="rounded-b-xl border border-t-0 border-orange-200/60 bg-white shadow-sm overflow-hidden">
                      <div className="flex items-center justify-between px-5 py-2.5 border-b border-orange-100 bg-gradient-to-r from-orange-50/80 to-amber-50/50">
                        <div className="flex items-center gap-2">
                          <Shield className="h-4 w-4 text-orange-600" />
                          <span className="text-xs font-semibold text-orange-800">
                            偏差检测 · v{drift.current_version}
                          </span>
                        </div>
                        <button
                          onClick={() => setShowDriftPanel(null)}
                          className="rounded-lg p-1 text-orange-400 hover:bg-orange-100 hover:text-orange-600 transition-colors"
                        >
                          <X className="h-3.5 w-3.5" />
                        </button>
                      </div>

                      {/* Drift items */}
                      <div className="px-5 py-3 space-y-1.5 max-h-52 overflow-y-auto">
                        {drift.drifts.map((d, i) => (
                          <div key={i} className="flex items-start gap-2.5 py-1">
                            <span className={cn(
                              "shrink-0 mt-1 inline-block w-2 h-2 rounded-full",
                              d.severity === "high" ? "bg-red-500" : d.severity === "medium" ? "bg-orange-400" : "bg-gray-300"
                            )} />
                            <span className="flex-1 text-sm text-gray-700 leading-snug">{d.description}</span>
                            <span className={cn(
                              "shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider",
                              d.severity === "high" ? "bg-red-50 text-red-600"
                                : d.severity === "medium" ? "bg-orange-50 text-orange-600"
                                : "bg-gray-50 text-gray-500"
                            )}>
                              {d.severity === "high" ? "高" : d.severity === "medium" ? "中" : "低"}
                            </span>
                          </div>
                        ))}
                      </div>

                      {/* Actions — two repair strategies */}
                      <div className="px-5 py-3 border-t border-orange-100 bg-orange-50/30">
                        <div className="grid grid-cols-2 gap-2">
                          {/* Option 1: Targeted fix */}
                          <div className="rounded-xl border border-orange-200/80 bg-white p-3">
                            <button
                              onClick={async () => {
                                setIsSuggesting(true);
                                setDriftSuggestions(null);
                                const result = await suggestCmd.execute({ root_path: project.rootPath });
                                setDriftSuggestions(result || "生成失败");
                                setIsSuggesting(false);
                              }}
                              disabled={isSuggesting}
                              className="w-full inline-flex items-center justify-center gap-1.5 rounded-lg bg-orange-500 px-3 py-2 text-xs font-medium text-white shadow-sm hover:bg-orange-600 transition-colors disabled:opacity-50"
                            >
                              {isSuggesting ? <Loader2 className="h-3 w-3 animate-spin" /> : <Sparkles className="h-3 w-3" />}
                              {isSuggesting ? "分析中…" : "修复偏差"}
                            </button>
                            <p className="mt-2 text-[11px] text-gray-400 leading-snug text-center">
                              只修改有问题的部分
                            </p>
                          </div>
                          {/* Option 2: Full rewrite with drift context */}
                          <div className="rounded-xl border border-gray-200/80 bg-white p-3">
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                setShowDriftPanel(null);
                                const driftSummary = drift.drifts
                                  .map((d) => `[${d.severity === "high" ? "高" : d.severity === "medium" ? "中" : "低"}] ${d.description}`)
                                  .join("；");
                                const ctx = `请确保新生成的内容解决以下偏差：${driftSummary}`;
                                setConfirmRegen({ id: project.id, name: project.name, rootPath: project.rootPath, driftContext: ctx });
                              }}
                              disabled={regeneratingId === project.id}
                              className="w-full inline-flex items-center justify-center gap-1.5 rounded-lg border border-gray-200 bg-white px-3 py-2 text-xs font-medium text-gray-700 hover:bg-gray-50 transition-colors disabled:opacity-50"
                            >
                              <RefreshCw className="h-3 w-3" />
                              全部重新生成
                            </button>
                            <p className="mt-2 text-[11px] text-gray-400 leading-snug text-center">
                              丢弃并重新生成整个文件
                            </p>
                          </div>
                        </div>
                      </div>

                      {/* Suggestions result */}
                      {driftSuggestions && (
                        <div className="border-t border-orange-100 bg-white">
                          <div className="px-5 py-3">
                            <div className="flex items-center gap-2 mb-2">
                              <Sparkles className="h-3.5 w-3.5 text-orange-500" />
                              <span className="text-xs font-semibold text-gray-700">AI 更新建议</span>
                            </div>
                            <pre className="whitespace-pre-wrap text-[13px] text-gray-600 font-mono leading-relaxed max-h-72 overflow-y-auto rounded-lg bg-gray-50/80 border border-gray-100 p-3">
                              {driftSuggestions}
                            </pre>
                          </div>
                          <div className="px-5 py-3 border-t border-gray-100 bg-gray-50/50 flex items-center gap-3">
                            <button
                              onClick={async () => {
                                setIsApplying(true);
                                setApplySuccess(false);
                                const result = await applyCmd.execute({
                                  root_path: project.rootPath,
                                  suggestions: driftSuggestions,
                                });
                                setIsApplying(false);
                                if (result) {
                                  setApplySuccess(true);
                                  const newDrift = await driftCmd.execute({ root_path: project.rootPath });
                                  if (newDrift) {
                                    setDriftMap(prev => ({ ...prev, [project.id]: newDrift }));
                                  }
                                  setDriftSuggestions(null);
                                  if (newDrift && !newDrift.is_stale) {
                                    setTimeout(() => setShowDriftPanel(null), 1500);
                                  }
                                }
                              }}
                              disabled={isApplying}
                              className="inline-flex items-center gap-1.5 rounded-lg bg-green-600 px-3.5 py-1.5 text-xs font-medium text-white shadow-sm hover:bg-green-700 transition-colors disabled:opacity-50"
                            >
                              {isApplying ? <Loader2 className="h-3 w-3 animate-spin" /> : <Check className="h-3 w-3" />}
                              {isApplying ? "正在更新…" : "应用到 AGENTS.md"}
                            </button>
                            <button
                              onClick={() => setDriftSuggestions(null)}
                              className="inline-flex items-center gap-1.5 rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-xs font-medium text-gray-600 hover:bg-gray-50 transition-colors"
                            >
                              忽略建议
                            </button>
                            {applySuccess && (
                              <span className="inline-flex items-center gap-1 text-xs text-green-600 font-medium">
                                <CheckCircle2 className="h-3 w-3" />
                                已更新，原文件备份为 .bak
                              </span>
                            )}
                            {applyCmd.error && (
                              <span className="text-xs text-red-500">{applyCmd.error}</span>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })()}

                {/* ── Inline compliance check panel ── */}
                {showCompliancePanel === project.id && complianceMap[project.id] && (() => {
                  const report = complianceMap[project.id];
                  const openViolations = report.violations.filter(v => v.status === "open");
                  return (
                    <div className="rounded-b-xl border border-t-0 border-blue-200/60 bg-white shadow-sm overflow-hidden">
                      <div className="flex items-center justify-between px-5 py-2.5 border-b border-blue-100 bg-gradient-to-r from-blue-50/80 to-indigo-50/50">
                        <div className="flex items-center gap-2">
                          <Activity className="h-4 w-4 text-blue-600" />
                          <span className="text-xs font-semibold text-blue-800">
                            合规检查 · {openViolations.length === 0 ? "全部通过" : `${openViolations.length} 项违规`}
                          </span>
                          {report.high > 0 && <span className="inline-flex items-center gap-0.5 text-[10px] text-red-600 font-medium"><span className="inline-block w-1.5 h-1.5 rounded-full bg-red-500" />{report.high} 高</span>}
                          {report.medium > 0 && <span className="inline-flex items-center gap-0.5 text-[10px] text-orange-600 font-medium"><span className="inline-block w-1.5 h-1.5 rounded-full bg-orange-400" />{report.medium} 中</span>}
                          {report.low > 0 && <span className="inline-flex items-center gap-0.5 text-[10px] text-gray-500 font-medium"><span className="inline-block w-1.5 h-1.5 rounded-full bg-gray-300" />{report.low} 低</span>}
                        </div>
                        <div className="flex items-center gap-1.5">
                          <button
                            onClick={async () => {
                              const result = await gitHooksCmd.execute({ root_path: project.rootPath });
                              if (result) {
                                setHookSetupResult(prev => ({ ...prev, [project.id]: result }));
                                setTimeout(() => setHookSetupResult(prev => { const next = { ...prev }; delete next[project.id]; return next; }), 4000);
                              }
                            }}
                            disabled={gitHooksCmd.isLoading}
                            className="inline-flex items-center gap-1 rounded-lg px-2 py-0.5 text-[10px] font-medium text-blue-500 hover:bg-blue-100 hover:text-blue-700 transition-colors border border-blue-200/50"
                            title="安装 Git pre-commit hook"
                          >
                            {gitHooksCmd.isLoading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Settings2 className="h-3 w-3" />}
                            Git Hooks
                          </button>
                          {hookSetupResult[project.id] && (
                            <span className="text-[10px] text-green-600 font-medium flex items-center gap-0.5"><CheckCircle2 className="h-3 w-3" />已安装</span>
                          )}
                          <button
                            onClick={() => handleRunComplianceCheck(project.id, project.rootPath)}
                            disabled={isCheckingCompliance === project.id}
                            className="rounded-lg p-1 text-blue-400 hover:bg-blue-100 hover:text-blue-600 transition-colors"
                            title="重新检查"
                          >
                            {isCheckingCompliance === project.id ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
                          </button>
                          <button
                            onClick={() => setShowCompliancePanel(null)}
                            className="rounded-lg p-1 text-blue-400 hover:bg-blue-100 hover:text-blue-600 transition-colors"
                          >
                            <X className="h-3.5 w-3.5" />
                          </button>
                        </div>
                      </div>

                      {openViolations.length === 0 ? (
                        <div className="px-5 py-8 text-center">
                          <CheckCircle2 className="mx-auto h-8 w-8 text-green-400 mb-2" />
                          <p className="text-sm font-medium text-green-700">所有合规检查通过</p>
                          <p className="text-xs text-gray-400 mt-1">未检测到违反治理规则的代码</p>
                        </div>
                      ) : (
                        <div className="px-5 py-3 space-y-1.5 max-h-64 overflow-y-auto">
                          {openViolations.map((v) => (
                            <div key={v.id} className="flex items-start gap-2.5 py-1.5 group/viol">
                              <span className={cn(
                                "shrink-0 mt-1 inline-block w-2 h-2 rounded-full",
                                v.severity === "high" ? "bg-red-500" : v.severity === "medium" ? "bg-orange-400" : "bg-gray-300"
                              )} />
                              <div className="flex-1 min-w-0">
                                <p className="text-sm text-gray-700 leading-snug">{v.description}</p>
                                <div className="flex items-center gap-2 mt-0.5">
                                  <span className="text-[10px] font-mono text-gray-400">{v.file_path}{v.line_number ? `:${v.line_number}` : ""}</span>
                                  <span className="text-[10px] font-medium text-blue-500">{v.rule_ref}</span>
                                </div>
                              </div>
                              <div className="flex items-center gap-1 shrink-0">
                                <span className={cn(
                                  "rounded px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider",
                                  v.severity === "high" ? "bg-red-50 text-red-600"
                                    : v.severity === "medium" ? "bg-orange-50 text-orange-600"
                                    : "bg-gray-50 text-gray-500"
                                )}>
                                  {v.severity === "high" ? "高" : v.severity === "medium" ? "中" : "低"}
                                </span>
                                <button
                                  onClick={() => handleDismissViolation(v.id, project.id)}
                                  className="opacity-0 group-hover/viol:opacity-100 rounded p-0.5 text-gray-300 hover:text-gray-500 hover:bg-gray-100 transition-all"
                                  title="忽略此违规"
                                >
                                  <X className="h-3 w-3" />
                                </button>
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  );
                })()}
              </div>
            );
          }))}
        </div>
      )}

      {/* ── File viewer modal ── */}
      {fileViewer && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-md">
          <div className="mx-4 w-full max-w-3xl max-h-[80vh] rounded-2xl bg-white/95 shadow-2xl backdrop-blur-xl ring-1 ring-black/5 flex flex-col overflow-hidden">
            {/* Header */}
            <div className="flex items-center justify-between px-5 py-4 border-b border-gray-100">
              <div className="flex items-center gap-3">
                <div className="rounded-lg bg-brand-50 p-2">
                  <FileCode2 className="h-4 w-4 text-brand-600" />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-gray-900">{fileViewer.fileName}</h2>
                  <p className="text-xs text-gray-400 font-mono truncate max-w-md">{fileViewer.projectName}</p>
                </div>
              </div>
              <button
                onClick={handleCloseFileViewer}
                className="rounded-lg p-1.5 text-gray-400 hover:bg-gray-100 hover:text-gray-600 transition-colors"
              >
                <X className="h-4 w-4" />
              </button>
            </div>
            {/* Content */}
            <div className="flex-1 overflow-auto p-5">
              {isLoadingFile ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="h-6 w-6 animate-spin text-gray-400" />
                </div>
              ) : fileContent ? (
                fileContent.exists ? (
                  <div className="overflow-auto bg-gray-50/80 rounded-xl border border-gray-100">
                    <pre className="text-sm text-gray-700 font-mono leading-relaxed">
                      {fileContent.content.split('\n').map((line, idx) => (
                        <div key={idx} className="flex hover:bg-gray-100/50">
                          <span className="select-none text-gray-400 text-right pr-3 pl-4 w-12 shrink-0 border-r border-gray-200 mr-3">
                            {idx + 1}
                          </span>
                          <span className="pr-4 whitespace-pre">{line}</span>
                        </div>
                      ))}
                    </pre>
                  </div>
                ) : (
                  <div className="flex flex-col items-center justify-center py-12 text-gray-400">
                    <X className="h-8 w-8 mb-2" />
                    <p className="text-sm">文件不存在</p>
                  </div>
                )
              ) : (
                <div className="flex items-center justify-center py-12 text-gray-400">
                  <p className="text-sm">加载失败</p>
                </div>
              )}
            </div>
            {/* Footer */}
            <div className="px-5 py-3 border-t border-gray-100 bg-gray-50/50 flex items-center justify-between">
              <span className="text-xs text-gray-400 font-mono truncate max-w-md">
                {fileContent?.path}
              </span>
              <button
                onClick={handleCloseFileViewer}
                className="rounded-lg px-4 py-1.5 text-xs font-medium text-gray-600 hover:bg-gray-100 transition-colors"
              >
                关闭
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Remove confirmation dialog ── */}
      {confirmRemove && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-md">
          <div className="mx-4 w-full max-w-sm rounded-2xl bg-white/95 p-7 shadow-2xl backdrop-blur-xl ring-1 ring-black/5 text-center">
            <div className="mx-auto w-fit rounded-2xl bg-gradient-to-br from-red-400 to-red-500 p-3 shadow-sm">
              <Trash2 className="h-7 w-7 text-white" />
            </div>
            <h2 className="mt-4 text-base font-semibold text-gray-900">
              移除「{confirmRemove.name}」？
            </h2>
            <p className="mt-2 text-sm text-gray-500 leading-relaxed">
              项目文件不会被删除，仅从 DocGuardian 中移除。
            </p>
            <div className="mt-6 flex gap-3">
              <button
                onClick={() => setConfirmRemove(null)}
                className="flex-1 rounded-xl px-4 py-2.5 text-sm font-medium text-gray-600 transition-colors hover:bg-gray-100"
              >
                取消
              </button>
              <button
                onClick={() => {
                  handleRemoveProject(confirmRemove.id);
                  setConfirmRemove(null);
                }}
                className="flex-1 inline-flex items-center justify-center gap-2 rounded-xl bg-red-600 px-4 py-2.5 text-sm font-medium text-white shadow-sm transition-all hover:bg-red-700 active:scale-[0.98]"
              >
                移除
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Regenerate confirmation dialog ── */}
      {confirmRegen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-md">
          <div className="mx-4 w-full max-w-sm rounded-2xl bg-white/95 p-7 shadow-2xl backdrop-blur-xl ring-1 ring-black/5 text-center">
            <div className="mx-auto w-fit rounded-2xl bg-gradient-to-br from-violet-400 to-purple-500 p-3 shadow-sm">
              <RefreshCw className="h-7 w-7 text-white" />
            </div>
            <h2 className="mt-4 text-base font-semibold text-gray-900">
              重新生成「{confirmRegen.name}」的 AGENTS.md？
            </h2>
            <p className="mt-2 text-sm text-gray-500 leading-relaxed">
              现有内容将被替换为全新生成的版本。
            </p>
            {confirmRegen.driftContext && (
              <p className="mt-2 text-xs text-violet-700 bg-violet-50 rounded-lg px-3 py-2 leading-relaxed text-left">
                生成后已知的偏差问题会自动填入优化栏，方便你进一步调整。
              </p>
            )}
            <div className="mt-6 flex gap-3">
              <button
                onClick={() => setConfirmRegen(null)}
                className="flex-1 rounded-xl px-4 py-2.5 text-sm font-medium text-gray-600 transition-colors hover:bg-gray-100"
              >
                取消
              </button>
              <button
                onClick={() => {
                  const { id, rootPath, driftContext } = confirmRegen;
                  setConfirmRegen(null);
                  handleRegenerateGovernance(id, rootPath, driftContext);
                }}
                className="flex-1 inline-flex items-center justify-center gap-2 rounded-xl bg-violet-600 px-4 py-2.5 text-sm font-medium text-white shadow-sm transition-all hover:bg-violet-700 active:scale-[0.98]"
              >
                重新生成
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Confirm dialog — "检测到项目未配置 AI 治理框架" */}
      {showConfirmDialog && wizardPlan && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-md">
          <div className="mx-4 w-full max-w-sm rounded-2xl bg-white/95 p-7 shadow-2xl backdrop-blur-xl ring-1 ring-black/5 text-center">
            <div className="mx-auto w-fit rounded-2xl bg-gradient-to-br from-amber-400 to-orange-500 p-3 shadow-sm">
              <ShieldAlert className="h-7 w-7 text-white" />
            </div>
            <h2 className="mt-4 text-base font-semibold text-gray-900">
              未检测到 AI 治理框架
            </h2>
            <p className="mt-2 text-sm text-gray-500 leading-relaxed">
              <strong>{wizardPlan.scan_result.project_name}</strong> 尚未配置 AGENTS.md，
              是否立即使用 AI 智能生成？
            </p>
            <p className="mt-2 text-xs text-gray-400">
              AI 将根据项目技术栈自动生成编码治理规则
            </p>
            <div className="mt-6 flex gap-3">
              <button
                onClick={() => {
                  setShowConfirmDialog(false);
                  setWizardPlan(null);
                }}
                className="flex-1 rounded-xl px-4 py-2.5 text-sm font-medium text-gray-600 transition-colors hover:bg-gray-100"
              >
                暂不需要
              </button>
              <button
                onClick={() => {
                  setShowConfirmDialog(false);
                  setAutoGenerate(true);
                  setShowWizard(true);
                }}
                className="flex-1 inline-flex items-center justify-center gap-2 rounded-xl bg-gray-900 px-4 py-2.5 text-sm font-medium text-white shadow-sm transition-all hover:bg-gray-800 active:scale-[0.98]"
              >
                <Sparkles className="h-4 w-4" />
                立即生成
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Full-screen Init Wizard */}
      {showWizard && wizardPlan && (
        <InitWizardModal
          plan={wizardPlan}
          onConfirm={handleConfirmInit}
          onSkip={() => { setShowWizard(false); setWizardPlan(null); setAutoGenerate(false); setRegenDriftContext(null); }}
          isLoading={confirmCmd.isLoading}
          initSuccess={initSuccess}
          onClose={handleCloseWizard}
          onUpdateFileContent={handleUpdateFileContent}
          autoGenerate={autoGenerate}
          onAutoGenerateConsumed={() => setAutoGenerate(false)}
          initialRefineFeedback={regenDriftContext ?? undefined}
        />
      )}
    </div>
  );
}

// --- Empty state ---

function EmptyState({ onAdd }: { onAdd: () => void }) {
  return (
    <div className="mx-auto max-w-lg py-8">
      {/* Hero illustration area */}
      <div className="text-center">
        <div className="mx-auto w-fit rounded-2xl bg-gradient-to-br from-brand-50 to-blue-50 p-5 shadow-sm ring-1 ring-brand-100/50">
          <Shield className="h-10 w-10 text-brand-500" />
        </div>
        <h2 className="mt-5 text-lg font-bold text-gray-900">开始管理你的 AI 编码治理</h2>
        <p className="mt-2 text-sm text-gray-500 leading-relaxed max-w-sm mx-auto">
          添加项目后，DocGuardian 将自动扫描技术栈并生成 AGENTS.md 治理框架，约束 AI 助手的行为边界
        </p>
      </div>

      {/* Value proposition cards */}
      <div className="mt-6 grid grid-cols-3 gap-3">
        <div className="rounded-xl bg-white border border-gray-100 p-3.5 shadow-sm">
          <div className="rounded-lg bg-green-50 p-1.5 w-fit">
            <ShieldCheck className="h-3.5 w-3.5 text-green-600" />
          </div>
          <p className="mt-2 text-xs font-semibold text-gray-800">自动生成治理规则</p>
          <p className="mt-0.5 text-[11px] text-gray-400 leading-snug">基于技术栈智能生成 AGENTS.md</p>
        </div>
        <div className="rounded-xl bg-white border border-gray-100 p-3.5 shadow-sm">
          <div className="rounded-lg bg-orange-50 p-1.5 w-fit">
            <Activity className="h-3.5 w-3.5 text-orange-600" />
          </div>
          <p className="mt-2 text-xs font-semibold text-gray-800">偏差实时检测</p>
          <p className="mt-0.5 text-[11px] text-gray-400 leading-snug">代码演进时自动发现规则偏差</p>
        </div>
        <div className="rounded-xl bg-white border border-gray-100 p-3.5 shadow-sm">
          <div className="rounded-lg bg-purple-50 p-1.5 w-fit">
            <Sparkles className="h-3.5 w-3.5 text-purple-600" />
          </div>
          <p className="mt-2 text-xs font-semibold text-gray-800">AI 一键修复</p>
          <p className="mt-0.5 text-[11px] text-gray-400 leading-snug">LLM 自动生成并应用更新建议</p>
        </div>
      </div>

      {/* CTA */}
      <div className="mt-6 text-center">
        <button
          onClick={onAdd}
          className="inline-flex items-center gap-2 rounded-xl bg-gray-900 px-5 py-2.5 text-sm font-medium text-white shadow-sm transition-all hover:bg-gray-800 active:scale-[0.98]"
        >
          <Plus className="h-4 w-4" />
          添加第一个项目
        </button>
        <p className="mt-2 text-[11px] text-gray-400">支持任何语言和框架的项目</p>
      </div>
    </div>
  );
}

// --- LLM Config type ---

interface LlmConfigData {
  provider: string;
  base_url: string;
  model: string;
  api_key: string | null;
}

const LLM_PROVIDERS = [
  { value: "deepseek", label: "DeepSeek", url: "https://api.deepseek.com/v1", model: "deepseek-chat" },
  { value: "openai", label: "OpenAI", url: "https://api.openai.com/v1", model: "gpt-4o" },
  { value: "doubao", label: "豆包 (火山方舟)", url: "https://ark.cn-beijing.volces.com/api/v3", model: "doubao-seed-2-0-pro-260215" },
  { value: "qwen", label: "通义千问 (阿里)", url: "https://dashscope.aliyuncs.com/compatible-mode/v1", model: "qwen-plus" },
  { value: "zhipu", label: "智谱 GLM", url: "https://open.bigmodel.cn/api/paas/v4", model: "glm-5" },
  { value: "ernie", label: "文心一言 (百度)", url: "https://qianfan.baidubce.com/v2", model: "ernie-4.0-8k" },
  { value: "spark", label: "讯飞星火", url: "https://spark-api-open.xf-yun.com/v1", model: "generalv3.5" },
  { value: "moonshot", label: "Moonshot (月之暗面)", url: "https://api.moonshot.cn/v1", model: "moonshot-v1-8k" },
  { value: "minimax", label: "MiniMax (稀宇)", url: "https://api.minimax.chat/v1", model: "abab6.5s-chat" },
  { value: "yi", label: "零一万物 (01.AI)", url: "https://api.lingyiwanwu.com/v1", model: "yi-large" },
  { value: "stepfun", label: "阶跃星辰", url: "https://api.stepfun.com/v1", model: "step-1-8k" },
  { value: "siliconflow", label: "SiliconFlow (硅基流动)", url: "https://api.siliconflow.cn/v1", model: "deepseek-ai/DeepSeek-V3" },
  { value: "claude", label: "Anthropic Claude", url: "https://api.anthropic.com/v1", model: "claude-sonnet-4-20250514" },
  { value: "ollama", label: "Ollama (本地)", url: "http://localhost:11434/v1", model: "llama3.1:8b" },
  { value: "openai_compatible", label: "自定义 (OpenAI 兼容)", url: "http://localhost:8080/v1", model: "custom-model" },
];

// --- Init Wizard Modal ---

function InitWizardModal({
  plan,
  onConfirm,
  onSkip,
  isLoading,
  initSuccess,
  onClose,
  onUpdateFileContent,
  autoGenerate,
  onAutoGenerateConsumed,
  initialRefineFeedback,
}: {
  plan: InitPlan;
  onConfirm: () => void;
  onSkip: () => void;
  isLoading: boolean;
  initSuccess: string[] | null;
  onClose: () => void;
  onUpdateFileContent: (fileIdx: number, content: string) => void;
  autoGenerate?: boolean;
  onAutoGenerateConsumed?: () => void;
  /** Pre-fill refine input after generation (e.g. drift summary from a stale project). */
  initialRefineFeedback?: string;
}) {
  const scan = plan.scan_result;
  // Only show AGENTS.md in the editor
  const agentsIdx = plan.files.findIndex((f) => f.rel_path === "AGENTS.md");
  const agentsFile = agentsIdx >= 0 ? plan.files[agentsIdx] : null;

  // LLM config state
  const getLlmConfigCmd = useTauriCommand<LlmConfigData | null>("get_llm_config");
  const saveLlmConfigCmd = useTauriCommand<void>("save_llm_config");

  const [llmConfig, setLlmConfig] = useState<LlmConfigData | null>(null);
  const [llmConfigLoaded, setLlmConfigLoaded] = useState(false);
  const [showLlmSetup, setShowLlmSetup] = useState(false);
  const [configForm, setConfigForm] = useState({
    provider: "deepseek",
    base_url: "https://api.deepseek.com/v1",
    model: "deepseek-chat",
    api_key: "",
  });
  const [testResult, setTestResult] = useState<"idle" | "testing" | "ok" | "fail">("idle");
  const [testError, setTestError] = useState<string | null>(null);

  // LLM generation state
  const [isGenerating, setIsGenerating] = useState(false);
  const [isRefining, setIsRefining] = useState(false);
  const [refineFeedback, setRefineFeedback] = useState("");
  const [llmError, setLlmError] = useState<{ hint: string; raw: string } | null>(null);

  // Selected text for targeted refinement
  const [selectedText, setSelectedText] = useState("");
  const refineInputRef = useRef<HTMLInputElement>(null);

  // Streaming state: accumulate chunks in a ref, update content periodically
  const streamBufferRef = useRef("");
  const [streamingContent, setStreamingContent] = useState<string | null>(null);

  // Reasoning/thinking state (for DeepSeek R1 and similar models)
  const reasoningBufferRef = useRef("");
  const [reasoningContent, setReasoningContent] = useState<string | null>(null);
  const [reasoningExpanded, setReasoningExpanded] = useState(false);

  // Editor refs for line-number sync and auto-scroll
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const gutterRef = useRef<HTMLDivElement>(null);
  const userScrolledRef = useRef(false);

  // Listen for LLM streaming chunk events
  useEffect(() => {
    const unlistenChunk = listen<string>("llm-chunk", (event) => {
      streamBufferRef.current += event.payload;
      setStreamingContent(streamBufferRef.current);
    });
    const unlistenReasoning = listen<string>("llm-reasoning", (event) => {
      reasoningBufferRef.current += event.payload;
      setReasoningContent(reasoningBufferRef.current);
      if (!reasoningExpanded) setReasoningExpanded(true);
    });
    return () => {
      unlistenChunk.then((f) => f());
      unlistenReasoning.then((f) => f());
    };
  }, []); // eslint-disable-line

  // Auto-scroll to bottom during streaming, unless user scrolled away
  useEffect(() => {
    if (streamingContent !== null && editorRef.current && !userScrolledRef.current) {
      editorRef.current.scrollTop = editorRef.current.scrollHeight;
    }
  }, [streamingContent]);

  // Reset user-scrolled flag when streaming starts
  useEffect(() => {
    if (streamingContent === "") {
      userScrolledRef.current = false;
    }
  }, [streamingContent]);

  // Sync line-number gutter scroll with textarea, and detect user scroll-away
  const handleEditorScroll = useCallback(() => {
    if (editorRef.current && gutterRef.current) {
      gutterRef.current.scrollTop = editorRef.current.scrollTop;
      // If streaming, check if user scrolled away from bottom
      if (streamingContent !== null) {
        const el = editorRef.current;
        const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 50;
        userScrolledRef.current = !atBottom;
      }
    }
  }, [streamingContent]);

  // Handle text selection in editor — track selected text
  const handleEditorSelect = useCallback(() => {
    if (!editorRef.current || streamingContent !== null) return;
    const el = editorRef.current;
    const sel = el.value.substring(el.selectionStart, el.selectionEnd).trim();
    setSelectedText(sel);
  }, [streamingContent]);

  // Send selected text to refine input
  const handleSendSelectionToRefine = useCallback(() => {
    if (!selectedText) return;
    const truncated = selectedText.length > 100
      ? selectedText.slice(0, 100) + "…"
      : selectedText;
    setRefineFeedback(`针对「${truncated}」：`);
    setSelectedText("");
    // Focus the refine input and place cursor at end
    setTimeout(() => {
      refineInputRef.current?.focus();
    }, 50);
  }, [selectedText]);

  // Interaction feedback toast
  const [toast, setToast] = useState<{ message: string; detail: string; type: "success" | "info" } | null>(null);
  const showToast = (message: string, detail: string, type: "success" | "info" = "success") => {
    setToast({ message, detail, type });
    setTimeout(() => setToast(null), 5000);
  };

  // Helper: count markdown sections
  const countSections = (md: string) => (md.match(/^##\s/gm) || []).length;

  // Load LLM config on mount
  useEffect(() => {
    (async () => {
      const cfg = await getLlmConfigCmd.execute();
      if (cfg) {
        setLlmConfig(cfg);
        setConfigForm({
          provider: cfg.provider,
          base_url: cfg.base_url,
          model: cfg.model,
          api_key: cfg.api_key || "",
        });
      }
      setLlmConfigLoaded(true);
    })();
  }, []);

  const handleProviderChange = (provider: string) => {
    const preset = LLM_PROVIDERS.find((p) => p.value === provider);
    if (preset) {
      setConfigForm({
        provider: preset.value,
        base_url: preset.url,
        model: preset.model,
        api_key: configForm.api_key,
      });
      setTestResult("idle");
      setTestError(null);
    }
  };

  const handleSaveLlmConfig = async () => {
    const cfg: LlmConfigData = {
      provider: configForm.provider,
      base_url: configForm.base_url,
      model: configForm.model,
      api_key: configForm.api_key || null,
    };
    await saveLlmConfigCmd.execute({ config: cfg });
    setLlmConfig(cfg);
    setShowLlmSetup(false);
    const providerLabel = LLM_PROVIDERS.find((p) => p.value === cfg.provider)?.label || cfg.provider;
    showToast("模型配置已保存", `${providerLabel} / ${cfg.model}`, "info");
  };

  const handleTestConnection = async () => {
    setTestResult("testing");
    setTestError(null);
    const cfg: LlmConfigData = {
      provider: configForm.provider,
      base_url: configForm.base_url,
      model: configForm.model,
      api_key: configForm.api_key || null,
    };
    try {
      await invoke("test_llm_connection", { config: cfg });
      setTestResult("ok");
    } catch (err) {
      setTestResult("fail");
      const { hint, raw } = parseApiError(String(err));
      setTestError(hint || raw);
    }
  };

  /** Extract the `message` field from a JSON error payload embedded in the raw error string */
  const extractApiMessage = (rawErr: string): string => {
    const match = rawErr.match(/"message"\s*:\s*"([^"]+)"/);
    return match ? match[1] : "";
  };

  /** Translate raw API errors into { hint, raw } — always preserve the original */
  const parseApiError = (rawErr: string): { hint: string; raw: string } => {
    const raw = rawErr;
    if (raw.includes("LLM 未配置") || (raw.includes("未配置") && !raw.includes("HTTP")))
      return { hint: "LLM 未配置 — 请先点击右上角「配置大模型」完成配置并保存。", raw };
    if (raw.includes("API Key 未配置"))
      return { hint: "API Key 为空 — 请在大模型配置面板填写 API Key 后保存。", raw };
    if (raw.includes("401") || raw.includes("Unauthorized") || raw.includes("authentication"))
      return { hint: "API Key 无效或已过期，请检查是否填写正确。", raw };
    if (raw.includes("403") || raw.includes("Forbidden"))
      return { hint: "API Key 权限不足，请确认该 Key 有权调用此模型。", raw };
    if (raw.includes("404") || raw.includes("model_not_found") || raw.includes("does not exist"))
      return { hint: "模型名称不存在，请检查「模型」字段是否填写正确（注意大小写）。", raw };
    if (raw.includes("1113") || raw.includes("余额不足") || raw.includes("无可用资源包"))
      return {
        hint: extractApiMessage(raw) || "账户余额不足或无可用资源包，请登录控制台充值后重试。",
        raw,
      };
    if (raw.includes("429") || raw.includes("rate_limit") || raw.includes("Too Many"))
      return { hint: extractApiMessage(raw) || "请求频率超限（429），请稍后重试或升级套餐。", raw };
    if (raw.includes("Connection refused") || raw.includes("connect error") || raw.includes("dns error"))
      return { hint: "无法连接服务器 — 请检查 API 地址是否正确，以及网络/代理设置。", raw };
    if (raw.includes("timeout") || raw.includes("timed out"))
      return { hint: "请求超时 — 服务器响应过慢，请稍后重试，或换用更快的模型。", raw };
    return { hint: extractApiMessage(raw), raw };
  };

  const handleStopGeneration = useCallback(async () => {
    try {
      await invoke("cancel_llm_generation");
    } catch {
      // ignore
    }
  }, []);

  const handleLlmGenerate = useCallback(async () => {
    setIsGenerating(true);
    setLlmError(null);
    streamBufferRef.current = "";
    setStreamingContent("");
    reasoningBufferRef.current = "";
    setReasoningContent(null);
    setReasoningExpanded(false);
    try {
      const content = await invoke<string>("generate_agents_md_llm", {
        root_path: scan.root_path,
      });
      if (agentsIdx >= 0) onUpdateFileContent(agentsIdx, content);
      setStreamingContent(null);
      streamBufferRef.current = "";
      const sections = countSections(content);
      if (initialRefineFeedback) {
        setRefineFeedback(initialRefineFeedback);
        showToast(
          "已生成",
          `${sections} 个章节 · 可在下方优化栏中继续调整`,
          "info"
        );
      } else {
        showToast(
          "已生成",
          `${sections} 个章节，${content.length.toLocaleString()} 字符`
        );
      }
    } catch (err) {
      setStreamingContent(null);
      streamBufferRef.current = "";
      setLlmError(parseApiError(String(err)));
    } finally {
      setIsGenerating(false);
    }
  }, [agentsIdx, plan.files, scan.root_path, initialRefineFeedback]);  // eslint-disable-line

  const handleRefine = async () => {
    if (!refineFeedback.trim()) return;
    const feedbackText = refineFeedback.trim();
    setIsRefining(true);
    setLlmError(null);
    streamBufferRef.current = "";
    setStreamingContent("");
    reasoningBufferRef.current = "";
    setReasoningContent(null);
    setReasoningExpanded(false);
    const prevContent = agentsFile?.content || "";
    try {
      const content = await invoke<string>("refine_agents_md", {
        root_path: scan.root_path,
        current_content: prevContent,
        user_feedback: feedbackText,
      });
      if (agentsIdx >= 0) onUpdateFileContent(agentsIdx, content);
      setStreamingContent(null);
      streamBufferRef.current = "";
      setRefineFeedback("");
      const sections = countSections(content);
      const charDiff = content.length - prevContent.length;
      showToast(
        "已优化",
        `${sections} 个章节（${charDiff >= 0 ? "+" : ""}${charDiff} 字符）`
      );
    } catch (err) {
      setStreamingContent(null);
      streamBufferRef.current = "";
      setLlmError(parseApiError(String(err)));
    } finally {
      setIsRefining(false);
    }
  };


  // Auto-generate on mount if requested
  const [autoGenTriggered, setAutoGenTriggered] = useState(false);
  useEffect(() => {
    if (autoGenerate && llmConfigLoaded && llmConfig && !autoGenTriggered && !isGenerating) {
      setAutoGenTriggered(true);
      onAutoGenerateConsumed?.();
      handleLlmGenerate();
    }
  }, [autoGenerate, llmConfigLoaded, llmConfig, autoGenTriggered, isGenerating, handleLlmGenerate]);

  // If autoGenerate requested but no LLM configured, show setup panel
  useEffect(() => {
    if (autoGenerate && llmConfigLoaded && !llmConfig && !autoGenTriggered) {
      setAutoGenTriggered(true);
      onAutoGenerateConsumed?.();
      setShowLlmSetup(true);
    }
  }, [autoGenerate, llmConfigLoaded, llmConfig, autoGenTriggered]);

  const providerLabel = llmConfig ? (LLM_PROVIDERS.find((p) => p.value === llmConfig.provider)?.label || llmConfig.provider) : "";

  // ── Success state ──
  if (initSuccess) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-md">
        <div className="mx-4 w-full max-w-md rounded-2xl bg-white/95 p-8 shadow-2xl text-center backdrop-blur-xl ring-1 ring-black/5">
          <div className="mx-auto w-fit rounded-full bg-green-50 p-4">
            <PartyPopper className="h-10 w-10 text-green-500" />
          </div>
          <h2 className="mt-5 text-lg font-semibold text-gray-900">
            治理框架已就绪
          </h2>
          <p className="mt-2 text-sm text-gray-500">
            已为 <strong>{scan.project_name}</strong> 写入以下文件
          </p>
          <div className="mt-5 space-y-2">
            {initSuccess.map((f) => (
              <div
                key={f}
                className="flex items-start gap-2.5 rounded-xl bg-green-50/80 px-4 py-3 text-left"
              >
                <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-green-500" />
                <div>
                  <span className="text-sm font-mono text-green-800">{f}</span>
                  <p className="mt-0.5 text-xs text-gray-500">
                    {f === "AGENTS.md"
                      ? "AI 编码宪法 — 定义权限边界、安全红线与验证规则，AI 助手会自动遵守"
                      : f === ".ai/progress.md"
                      ? "任务追踪文件 — 记录当前开发进度与待办事项，AI 助手会自动读写"
                      : f === ".docguardian.toml"
                      ? "项目配置文件 — DocGuardian 的监控策略与功能开关"
                      : "治理相关文件"}
                  </p>
                </div>
              </div>
            ))}
          </div>
          <p className="mt-4 text-xs text-gray-400">
            AI 编码助手将自动读取这些治理规则
          </p>
          <button onClick={onClose} className="mt-6 w-full inline-flex items-center justify-center gap-2 rounded-xl bg-gray-900 px-5 py-2.5 text-sm font-medium text-white transition-colors hover:bg-gray-800">
            <ShieldCheck className="h-4 w-4" />
            完成
          </button>
        </div>
      </div>
    );
  }

  // ── Normal wizard state — full-screen AGENTS.md editor ──
  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-white">

      {/* Toast notification */}
      {toast && (
        <div className={cn(
          "fixed top-5 left-1/2 z-[60] -translate-x-1/2 animate-in fade-in slide-in-from-top-2 duration-300",
          "flex items-center gap-3 rounded-xl px-5 py-3 shadow-lg ring-1",
          toast.type === "success"
            ? "bg-green-50 text-green-800 ring-green-200/60"
            : "bg-blue-50 text-blue-800 ring-blue-200/60"
        )}>
          {toast.type === "success" ? (
            <CheckCircle2 className="h-4 w-4 shrink-0 text-green-500" />
          ) : (
            <Bot className="h-4 w-4 shrink-0 text-blue-500" />
          )}
          <div>
            <p className="text-sm font-medium">{toast.message}</p>
            <p className="text-xs opacity-70">{toast.detail}</p>
          </div>
          <button onClick={() => setToast(null)} className="ml-2 rounded-md p-0.5 opacity-40 hover:opacity-100 transition-opacity">
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      )}

      {/* Header toolbar */}
      <div className="flex items-center justify-between border-b border-gray-200/60 bg-gray-50/80 px-5 py-2.5 shrink-0">
        <div className="flex items-center gap-3">
          <div className="rounded-xl bg-gradient-to-br from-amber-400 to-orange-500 p-2 shadow-sm">
            <ShieldCheck className="h-4 w-4 text-white" />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-gray-900">AGENTS.md</h2>
            <div className="flex items-center gap-3 text-[11px] text-gray-500">
              <span>{scan.project_name}</span>
              <span>{scan.languages.join(", ") || "未知"}</span>
              <span>{scan.frameworks.join(", ") || "无框架"}</span>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {/* LLM error inline */}
          {llmError && (
            <div className="rounded-lg bg-red-50 px-3 py-2 ring-1 ring-red-100 max-w-lg space-y-1">
              {llmError.hint && (
                <p className="text-[11px] text-red-700 font-medium leading-snug">
                  ⚠ {llmError.hint}
                </p>
              )}
              <p className="text-[10px] text-red-400 font-mono break-all leading-snug select-all">
                {llmError.raw}
              </p>
            </div>
          )}
          {/* Regenerate */}
          {llmConfig && (
            <button
              onClick={handleLlmGenerate}
              disabled={isGenerating}
              className="inline-flex items-center gap-1.5 rounded-lg bg-gradient-to-r from-violet-500 to-blue-500 px-3 py-1.5 text-xs font-medium text-white shadow-sm transition-all hover:shadow-md active:scale-[0.98] disabled:opacity-60"
            >
              {isGenerating ? (
                <>
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  生成中…
                </>
              ) : (
                <>
                  <Sparkles className="h-3.5 w-3.5" />
                  重新生成
                </>
              )}
            </button>
          )}
          {/* LLM model badge */}
          {llmConfigLoaded && (
            <button
              onClick={() => setShowLlmSetup(!showLlmSetup)}
              className={cn(
                "inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-all",
                llmConfig
                  ? "bg-white text-gray-700 shadow-sm ring-1 ring-gray-200/80 hover:shadow"
                  : "bg-purple-50 text-purple-600 hover:bg-purple-100"
              )}
            >
              <Bot className="h-3.5 w-3.5" />
              {llmConfig ? providerLabel : "配置大模型"}
              <Settings2 className="h-3 w-3 opacity-40" />
            </button>
          )}
          <button onClick={onSkip} className="rounded-lg p-1.5 text-gray-400 hover:bg-gray-200/60 transition-colors">
            <X className="h-4 w-4" />
          </button>
        </div>
      </div>

      {/* LLM Config Panel (collapsible) */}
      {showLlmSetup && (
        <div className="border-b border-gray-200/60 bg-gray-50/60 px-5 py-4 shrink-0">
          <div className="mx-auto max-w-3xl flex items-start gap-4">
            <div className="flex-1 space-y-2.5">
              <div className="flex items-center gap-3">
                <label className="text-xs font-medium text-gray-500 w-16 shrink-0">服务商</label>
                <select
                  value={configForm.provider}
                  onChange={(e) => handleProviderChange(e.target.value)}
                  className="rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-sm shadow-sm focus:border-blue-400 focus:outline-none focus:ring-2 focus:ring-blue-400/20"
                >
                  {LLM_PROVIDERS.map((p) => (
                    <option key={p.value} value={p.value}>{p.label}</option>
                  ))}
                </select>
              </div>
              <div className="flex items-center gap-3">
                <label className="text-xs font-medium text-gray-500 w-16 shrink-0">Base URL</label>
                <input
                  type="text"
                  value={configForm.base_url}
                  onChange={(e) => setConfigForm({ ...configForm, base_url: e.target.value })}
                  className="flex-1 rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-sm font-mono shadow-sm focus:border-blue-400 focus:outline-none focus:ring-2 focus:ring-blue-400/20"
                />
              </div>
              <div className="flex items-center gap-3">
                <label className="text-xs font-medium text-gray-500 w-16 shrink-0">模型</label>
                <div className="flex-1">
                  <input
                    type="text"
                    value={configForm.model}
                    onChange={(e) => setConfigForm({ ...configForm, model: e.target.value })}
                    placeholder={configForm.provider === "doubao" ? "模型名或接入点 ID (ep-xxx)" : ""}
                    className="w-full rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-sm font-mono shadow-sm focus:border-blue-400 focus:outline-none focus:ring-2 focus:ring-blue-400/20"
                  />
                  {configForm.provider === "doubao" && (
                    <p className="mt-1 text-[10px] text-gray-400">
                      填写模型名（如 doubao-seed-2-0-pro-260215）或方舟控制台的接入点 ID（ep-xxx）
                    </p>
                  )}
                </div>
              </div>
              {configForm.provider !== "ollama" && (
                <div className="flex items-center gap-3">
                  <label className="text-xs font-medium text-gray-500 w-16 shrink-0">API Key</label>
                  <input
                    type="password"
                    value={configForm.api_key}
                    onChange={(e) => setConfigForm({ ...configForm, api_key: e.target.value })}
                    placeholder={configForm.provider === "doubao" ? "火山方舟 API Key" : "sk-..."}
                    className="flex-1 rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-sm font-mono shadow-sm focus:border-blue-400 focus:outline-none focus:ring-2 focus:ring-blue-400/20"
                  />
                </div>
              )}
            </div>
            <div className="flex flex-col gap-2 shrink-0">
              <button onClick={handleTestConnection} disabled={testResult === "testing"} className="inline-flex items-center justify-center gap-1.5 rounded-lg bg-white px-4 py-1.5 text-xs font-medium text-gray-700 shadow-sm ring-1 ring-gray-200 transition-all hover:shadow disabled:opacity-50">
                {testResult === "testing" ? <Loader2 className="h-3 w-3 animate-spin" /> : <Zap className="h-3 w-3" />}
                测试连接
              </button>
              <button onClick={handleSaveLlmConfig} className="inline-flex items-center justify-center gap-1.5 rounded-lg bg-gray-900 px-4 py-1.5 text-xs font-medium text-white shadow-sm transition-all hover:bg-gray-800">
                <Check className="h-3 w-3" /> 保存
              </button>
              {testResult === "ok" && (
                <div className="rounded-lg bg-green-50 px-3 py-1.5 text-center">
                  <span className="text-[11px] text-green-600 font-medium">连接成功</span>
                </div>
              )}
              {testResult === "fail" && (
                <div className="rounded-lg bg-red-50 px-2 py-1.5 text-center max-w-[160px]">
                  <span className="text-[11px] text-red-500 font-medium">连接失败</span>
                  {testError && (
                    <p className="mt-0.5 text-[9px] text-red-400 break-all leading-tight">{testError}</p>
                  )}
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Full-screen editor area */}
      <div className="flex-1 overflow-hidden flex flex-col">
        {/* Streaming status bar */}
        {(isGenerating || isRefining) && streamingContent !== null && (
          <div className="flex items-center gap-2 px-5 py-1.5 bg-violet-50 border-b border-violet-100 text-[11px] text-violet-600 shrink-0">
            <Loader2 className="h-3 w-3 animate-spin" />
            <span>{isRefining ? "AI 正在优化" : providerLabel + " 正在生成"}</span>
            <span className="text-violet-400">·</span>
            <span className="font-mono">{streamingContent.length} 字符</span>
            <button
              onClick={handleStopGeneration}
              className="ml-auto inline-flex items-center gap-1 rounded px-2 py-0.5 text-[11px] font-medium text-red-500 hover:bg-red-50 transition-colors"
            >
              <StopCircle className="h-3 w-3" />
              停止生成
            </button>
          </div>
        )}

        {/* Reasoning/thinking panel (DeepSeek R1, etc.) */}
        {reasoningContent && (
          <div className="border-b border-amber-100 bg-amber-50/50 shrink-0">
            <button
              onClick={() => setReasoningExpanded(!reasoningExpanded)}
              className="flex w-full items-center gap-2 px-5 py-1.5 text-[11px] text-amber-700 hover:bg-amber-50 transition-colors"
            >
              <Brain className="h-3 w-3" />
              <span className="font-medium">AI 推理过程</span>
              <span className="text-amber-500">·</span>
              <span className="font-mono">{reasoningContent.length} 字符</span>
              {reasoningExpanded ? (
                <ChevronUp className="ml-auto h-3 w-3" />
              ) : (
                <ChevronDown className="ml-auto h-3 w-3" />
              )}
            </button>
            {reasoningExpanded && (
              <pre className="max-h-48 overflow-auto px-5 pb-3 text-[12px] leading-relaxed text-amber-800/70 italic whitespace-pre-wrap">
                {reasoningContent}
              </pre>
            )}
          </div>
        )}

        {/* Editor with line numbers */}
        {(() => {
          const displayContent = streamingContent !== null ? streamingContent : (agentsFile?.content || "");
          const lines = displayContent.split("\n");
          const lineCount = lines.length || 1;
          return (
            <div className="flex-1 flex overflow-hidden bg-white">
              {/* Line number gutter */}
              <div
                ref={gutterRef}
                className="shrink-0 overflow-hidden select-none border-r border-gray-100 bg-gray-50/80 py-4 text-right"
                style={{ width: `${Math.max(3, String(lineCount).length + 1.5)}em` }}
              >
                {Array.from({ length: lineCount }, (_, i) => (
                  <div
                    key={i}
                    className="px-3 font-mono text-[13px] leading-relaxed text-gray-300"
                  >
                    {i + 1}
                  </div>
                ))}
              </div>
              {/* Textarea */}
              <div className="relative flex-1 flex">
                <textarea
                  ref={editorRef}
                  value={displayContent}
                  onChange={(e) => {
                    if (streamingContent !== null) return;
                    if (agentsIdx >= 0) onUpdateFileContent(agentsIdx, e.target.value);
                  }}
                  onScroll={handleEditorScroll}
                  onSelect={handleEditorSelect}
                  onMouseUp={handleEditorSelect}
                  readOnly={streamingContent !== null}
                  className={cn(
                    "flex-1 resize-none overflow-auto py-4 pl-4 pr-8 font-mono text-[13px] leading-relaxed bg-white focus:outline-none selection:bg-violet-100",
                    streamingContent !== null ? "text-gray-500" : "text-gray-700"
                  )}
                  spellCheck={false}
                  placeholder="AGENTS.md 内容将在此处显示，点击上方「重新生成」或配置大模型后自动生成…"
                />
                {/* Floating "send to refine" button when text is selected */}
                {selectedText && llmConfig && !isGenerating && !isRefining && (
                  <button
                    onClick={handleSendSelectionToRefine}
                    className="absolute bottom-4 right-4 z-10 inline-flex items-center gap-1.5 rounded-lg bg-violet-500 px-3 py-1.5 text-xs font-medium text-white shadow-lg transition-all hover:bg-violet-600 active:scale-95 animate-in fade-in zoom-in-95 duration-150"
                  >
                    <Send className="h-3 w-3" />
                    优化选中内容
                  </button>
                )}
              </div>
            </div>
          );
        })()}
      </div>

      {/* Bottom bar: refinement + action */}
      <div className="border-t border-gray-200/60 bg-gray-50/80 px-5 py-3 shrink-0">
        <div className="flex items-center gap-3">
          {llmConfig && (
            <div className="relative flex-1">
              <input
                ref={refineInputRef}
                type="text"
                value={refineFeedback}
                onChange={(e) => setRefineFeedback(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    handleRefine();
                  }
                }}
                placeholder={selectedText ? "选中文本后点击「优化选中内容」…" : "输入优化意见，如：增加安全约束、补充测试规范…"}
                className="w-full rounded-xl border border-gray-200 bg-white py-2.5 pl-4 pr-20 text-sm shadow-sm focus:border-violet-400 focus:outline-none focus:ring-2 focus:ring-violet-400/20"
                disabled={isRefining || isGenerating}
              />
              <div className="absolute right-1.5 top-1/2 -translate-y-1/2">
                <button
                  onClick={handleRefine}
                  disabled={isRefining || !refineFeedback.trim() || isGenerating}
                  className={cn(
                    "inline-flex items-center gap-1 rounded-lg px-3 py-1.5 text-xs font-medium transition-all",
                    refineFeedback.trim()
                      ? "bg-violet-500 text-white shadow-sm hover:bg-violet-600"
                      : "bg-gray-100 text-gray-400"
                  )}
                >
                  {isRefining ? (
                    <Loader2 className="h-3 w-3 animate-spin" />
                  ) : (
                    <Send className="h-3 w-3" />
                  )}
                  优化
                </button>
              </div>
            </div>
          )}
          {!llmConfig && llmConfigLoaded && (
            <button
              onClick={() => setShowLlmSetup(true)}
              className="flex-1 inline-flex items-center justify-center gap-2 rounded-xl border border-dashed border-gray-300 px-3 py-2.5 text-[13px] text-gray-500 transition-colors hover:border-violet-300 hover:text-violet-600 hover:bg-violet-50/50"
            >
              <Bot className="h-4 w-4" />
              配置大模型以启用 AI 生成与优化
            </button>
          )}
          <button onClick={onSkip} className="shrink-0 rounded-xl px-4 py-2.5 text-sm text-gray-500 transition-colors hover:bg-gray-200/60 hover:text-gray-700">
            取消
          </button>
          <button
            onClick={onConfirm}
            disabled={isLoading || isGenerating || isRefining}
            className="shrink-0 inline-flex items-center gap-2 rounded-xl bg-gray-900 px-6 py-2.5 text-sm font-medium text-white shadow-sm transition-all hover:bg-gray-800 active:scale-[0.98] disabled:opacity-50"
          >
            {isLoading ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                写入中…
              </>
            ) : (
              <>
                <ShieldCheck className="h-4 w-4" />
                写入治理框架
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
