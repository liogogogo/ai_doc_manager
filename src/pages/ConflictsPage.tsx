import { AlertTriangle, Check, X, ExternalLink } from "lucide-react";
import { cn } from "@/lib/utils";

interface Conflict {
  id: string;
  filePath: string;
  lineRange: string;
  description: string;
  suggestion: string;
  commitHash: string;
  severity: "high" | "medium" | "low";
  status: "open" | "resolved" | "dismissed";
}

const mockConflicts: Conflict[] = [
  {
    id: "c1",
    filePath: "docs/design/auth.md",
    lineRange: "L42-L58",
    description: "文档描述 JWT Token 有效期为 24h，但代码中 TOKEN_EXPIRY 已改为 2h",
    suggestion: "将文档第 42 行 '有效期 24 小时' 更新为 '有效期 2 小时'",
    commitHash: "a3f21bc",
    severity: "high",
    status: "open",
  },
  {
    id: "c2",
    filePath: "docs/design/im-api-spec.md",
    lineRange: "L120-L135",
    description: "消息发送接口的 request body 中 content 字段已改名为 payload",
    suggestion: "更新 API 文档中的字段名 content → payload",
    commitHash: "e7d94a1",
    severity: "high",
    status: "open",
  },
  {
    id: "c3",
    filePath: "docs/design/database.md",
    lineRange: "L88",
    description: "文档中 users 表缺少 deleted_at 字段（已在 migration 中添加）",
    suggestion: "在 users 表字段列表中补充 deleted_at TIMESTAMP NULL",
    commitHash: "1bc45de",
    severity: "medium",
    status: "open",
  },
];

const severityConfig = {
  high: { badge: "badge-danger", label: "高" },
  medium: { badge: "badge-warning", label: "中" },
  low: { badge: "badge-info", label: "低" },
};

export function ConflictsPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">知识冲突检测</h1>
        <div className="flex items-center gap-2">
          <span className="text-sm text-gray-500">
            {mockConflicts.filter((c) => c.status === "open").length} 个未解决
          </span>
          <button className="btn-primary">
            <AlertTriangle className="h-4 w-4" />
            扫描冲突
          </button>
        </div>
      </div>

      <div className="space-y-3">
        {mockConflicts.map((conflict) => (
          <div key={conflict.id} className="card p-5">
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <span className={cn(severityConfig[conflict.severity].badge)}>
                    {severityConfig[conflict.severity].label}
                  </span>
                  <code className="text-sm font-mono text-gray-700">
                    {conflict.filePath}:{conflict.lineRange}
                  </code>
                  <code className="text-xs text-gray-400">@{conflict.commitHash}</code>
                </div>
                <p className="mt-2 text-sm text-gray-700">{conflict.description}</p>
                <div className="mt-2 rounded-md bg-green-50 border border-green-200 px-3 py-2">
                  <p className="text-sm text-green-800">
                    <span className="font-medium">建议修正：</span>{conflict.suggestion}
                  </p>
                </div>
              </div>
              <div className="flex shrink-0 gap-1.5">
                <button className="rounded-lg border border-gray-200 p-2 text-gray-400 hover:bg-green-50 hover:text-green-600 hover:border-green-200 transition-colors" title="接受修正">
                  <Check className="h-4 w-4" />
                </button>
                <button className="rounded-lg border border-gray-200 p-2 text-gray-400 hover:bg-red-50 hover:text-red-600 hover:border-red-200 transition-colors" title="忽略">
                  <X className="h-4 w-4" />
                </button>
                <button className="rounded-lg border border-gray-200 p-2 text-gray-400 hover:bg-blue-50 hover:text-blue-600 hover:border-blue-200 transition-colors" title="在编辑器中打开">
                  <ExternalLink className="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
