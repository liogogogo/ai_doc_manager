import { Scissors, Trash2, Archive, ArrowRight } from "lucide-react";
import { cn } from "@/lib/utils";

interface PruneItem {
  id: string;
  filePath: string;
  lineRange: string;
  snippet: string;
  category: "linter" | "script" | "stale";
  replacement: string;
}

const categoryConfig = {
  linter: { badge: "badge-info", label: "可被 Linter 替代" },
  script: { badge: "badge-warning", label: "可被脚本替代" },
  stale: { badge: "badge-danger", label: "已过时" },
};

const mockItems: PruneItem[] = [
  {
    id: "p1",
    filePath: "docs/style-guide.md",
    lineRange: "L12-L28",
    snippet: "所有代码必须使用 2 空格缩进，禁止使用 Tab...",
    category: "linter",
    replacement: "建议删除此段，已由 .prettierrc 中 tabWidth: 2 覆盖",
  },
  {
    id: "p2",
    filePath: "docs/deploy-guide.md",
    lineRange: "L1-L45",
    snippet: "部署步骤：1. 登录服务器 2. 拉取代码 3. 执行 build...",
    category: "script",
    replacement: "建议删除此文档，转为 Makefile deploy target 或 GitHub Actions",
  },
  {
    id: "p3",
    filePath: "docs/design/payment-v1.md",
    lineRange: "全文",
    snippet: "V1 支付接口设计：使用同步回调模式...",
    category: "stale",
    replacement: "建议添加 [DEPRECATED] 标记，当前版本为 payment-v2.md",
  },
];

export function PrunerPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">冗余文档清理</h1>
        <button className="btn-primary">
          <Scissors className="h-4 w-4" />
          全量扫描
        </button>
      </div>

      <div className="space-y-3">
        {mockItems.map((item) => (
          <div key={item.id} className="card p-5">
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1">
                <div className="flex items-center gap-2 mb-2">
                  <span className={cn(categoryConfig[item.category].badge)}>
                    {categoryConfig[item.category].label}
                  </span>
                  <code className="text-sm font-mono text-gray-700">
                    {item.filePath}:{item.lineRange}
                  </code>
                </div>

                <div className="rounded-md bg-gray-50 border border-gray-200 px-3 py-2 mb-2">
                  <p className="text-sm text-gray-600 italic">"{item.snippet}"</p>
                </div>

                <div className="flex items-center gap-2 text-sm">
                  <ArrowRight className="h-3.5 w-3.5 text-gray-400" />
                  <span className="text-green-700">{item.replacement}</span>
                </div>
              </div>

              <div className="flex shrink-0 gap-1.5">
                <button className="rounded-lg border border-gray-200 p-2 text-gray-400 hover:bg-red-50 hover:text-red-600 hover:border-red-200 transition-colors" title="删除此段">
                  <Trash2 className="h-4 w-4" />
                </button>
                <button className="rounded-lg border border-gray-200 p-2 text-gray-400 hover:bg-amber-50 hover:text-amber-600 hover:border-amber-200 transition-colors" title="标记为 DEPRECATED">
                  <Archive className="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
