import { useNavigate } from "react-router-dom";
import { ChevronDown, FolderOpen } from "lucide-react";
import { useProjectStore } from "@/stores/projectStore";

export function Header() {
  const currentProject = useProjectStore((s) => s.currentProject);
  const navigate = useNavigate();

  return (
    <header
      className="flex h-14 items-center justify-between border-b border-gray-200 bg-white px-6"
      data-tauri-drag-region
    >
      <div className="flex items-center gap-3">
        <button
          onClick={() => navigate("/projects")}
          className="btn-secondary text-sm"
        >
          <FolderOpen className="h-4 w-4" />
          {currentProject?.name ?? "选择项目"}
          <ChevronDown className="h-3.5 w-3.5 text-gray-400" />
        </button>
      </div>
      <div className="flex items-center gap-2">
        <span className="text-xs text-gray-400">v0.1.0</span>
      </div>
    </header>
  );
}
