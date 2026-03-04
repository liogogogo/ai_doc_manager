import { BookOpen, Check, X, Lightbulb } from "lucide-react";

interface RuleSuggestion {
  id: string;
  pattern: string;
  frequency: number;
  suggestion: string;
  targetFile: string;
  goldenExample?: string;
  status: "proposed" | "accepted" | "rejected";
}

const mockSuggestions: RuleSuggestion[] = [
  {
    id: "r1",
    pattern: "Redis 分布式锁未设置超时时间",
    frequency: 5,
    suggestion: "在 AGENTS.md 中新增：所有 Redis 分布式锁必须设置 TTL，且 TTL 不超过 30s",
    targetFile: "AGENTS.md",
    goldenExample: `// ✅ 正确示例
const lock = await redis.set(key, value, 'EX', 30, 'NX');

// ❌ 错误示例
const lock = await redis.set(key, value, 'NX');`,
    status: "proposed",
  },
  {
    id: "r2",
    pattern: "API 响应未包装统一格式",
    frequency: 3,
    suggestion: "在 best_practices/api-response.md 中新增 Golden Example",
    targetFile: "best_practices/api-response.md",
    goldenExample: `// ✅ 统一响应格式
return { code: 0, data: result, message: "ok" };

// ❌ 直接返回裸数据
return result;`,
    status: "proposed",
  },
];

export function RulesPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">隐式规则提取</h1>
        <button className="btn-primary">
          <Lightbulb className="h-4 w-4" />
          扫描失败日志
        </button>
      </div>

      <div className="space-y-4">
        {mockSuggestions.map((rule) => (
          <div key={rule.id} className="card p-5">
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1">
                <div className="flex items-center gap-2 mb-2">
                  <BookOpen className="h-4 w-4 text-blue-500" />
                  <span className="text-sm font-medium text-gray-900">{rule.pattern}</span>
                  <span className="badge-warning">出现 {rule.frequency} 次</span>
                </div>

                <p className="text-sm text-gray-600 mb-2">{rule.suggestion}</p>
                <p className="text-xs text-gray-400">
                  目标文件：<code className="font-mono">{rule.targetFile}</code>
                </p>

                {rule.goldenExample && (
                  <pre className="mt-3 rounded-lg bg-gray-900 p-4 text-sm text-gray-100 overflow-x-auto">
                    <code>{rule.goldenExample}</code>
                  </pre>
                )}
              </div>

              <div className="flex shrink-0 gap-1.5">
                <button className="rounded-lg border border-gray-200 p-2 text-gray-400 hover:bg-green-50 hover:text-green-600 hover:border-green-200 transition-colors" title="接受并写入">
                  <Check className="h-4 w-4" />
                </button>
                <button className="rounded-lg border border-gray-200 p-2 text-gray-400 hover:bg-red-50 hover:text-red-600 hover:border-red-200 transition-colors" title="拒绝">
                  <X className="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
