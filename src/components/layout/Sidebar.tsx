import { NavLink } from "react-router-dom";
import {
  LayoutDashboard,
  FolderOpen,
  Recycle,
  AlertTriangle,
  BookOpen,
  Scissors,
  Settings,
  Shield,
} from "lucide-react";
import { cn } from "@/lib/utils";

const navItems = [
  { to: "/projects", label: "项目管理", icon: FolderOpen },
  { to: "/dashboard", label: "仪表盘", icon: LayoutDashboard },
  { to: "/gc", label: "记忆回收", icon: Recycle },
  { to: "/conflicts", label: "冲突检测", icon: AlertTriangle },
  { to: "/rules", label: "规则提取", icon: BookOpen },
  { to: "/pruner", label: "冗余清理", icon: Scissors },
];

export function Sidebar() {
  return (
    <aside className="flex w-56 flex-col border-r border-gray-200 bg-white">
      {/* Logo */}
      <div className="flex h-14 items-center gap-2.5 border-b border-gray-200 px-5" data-tauri-drag-region>
        <Shield className="h-6 w-6 text-brand-600" />
        <span className="text-base font-semibold tracking-tight text-gray-900">
          DocGuardian
        </span>
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-1 px-3 py-4">
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-brand-50 text-brand-700"
                  : "text-gray-600 hover:bg-gray-100 hover:text-gray-900",
              )
            }
          >
            <item.icon className="h-4.5 w-4.5" />
            {item.label}
          </NavLink>
        ))}
      </nav>

      {/* Bottom settings link */}
      <div className="border-t border-gray-200 px-3 py-3">
        <NavLink
          to="/settings"
          className={({ isActive }) =>
            cn(
              "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
              isActive
                ? "bg-brand-50 text-brand-700"
                : "text-gray-600 hover:bg-gray-100 hover:text-gray-900",
            )
          }
        >
          <Settings className="h-4.5 w-4.5" />
          设置
        </NavLink>
      </div>
    </aside>
  );
}
