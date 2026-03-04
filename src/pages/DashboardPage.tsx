import {
  FileText,
  AlertTriangle,
  BookOpen,
  Clock,
  TrendingUp,
  ShieldAlert,
} from "lucide-react";
import { cn } from "@/lib/utils";

interface MetricCardProps {
  label: string;
  value: string | number;
  icon: React.ElementType;
  color: "blue" | "red" | "amber" | "green";
}

const colorMap = {
  blue: "bg-blue-50 text-blue-600",
  red: "bg-red-50 text-red-600",
  amber: "bg-amber-50 text-amber-600",
  green: "bg-green-50 text-green-600",
};

function MetricCard({ label, value, icon: Icon, color }: MetricCardProps) {
  return (
    <div className="card flex items-center gap-4 p-5">
      <div className={cn("rounded-lg p-2.5", colorMap[color])}>
        <Icon className="h-5 w-5" />
      </div>
      <div>
        <p className="text-2xl font-semibold tabular-nums">{value}</p>
        <p className="text-sm text-gray-500">{label}</p>
      </div>
    </div>
  );
}

function HealthScore({ score }: { score: number }) {
  const circumference = 2 * Math.PI * 42;
  const offset = circumference - (score / 100) * circumference;
  const color =
    score >= 80 ? "text-green-500" : score >= 60 ? "text-amber-500" : "text-red-500";

  return (
    <div className="card flex flex-col items-center justify-center p-8">
      <div className="relative h-32 w-32">
        <svg className="h-32 w-32 -rotate-90" viewBox="0 0 96 96">
          <circle
            cx="48" cy="48" r="42"
            fill="none" stroke="currentColor"
            className="text-gray-100" strokeWidth="8"
          />
          <circle
            cx="48" cy="48" r="42"
            fill="none" stroke="currentColor"
            className={color} strokeWidth="8"
            strokeLinecap="round"
            strokeDasharray={circumference}
            strokeDashoffset={offset}
          />
        </svg>
        <div className="absolute inset-0 flex items-center justify-center">
          <span className="text-3xl font-bold tabular-nums">{score}</span>
        </div>
      </div>
      <p className="mt-3 text-sm font-medium text-gray-600">文档健康度</p>
    </div>
  );
}

interface Activity {
  id: string;
  type: "gc" | "conflict" | "rule" | "prune";
  message: string;
  time: string;
}

const mockActivities: Activity[] = [
  { id: "1", type: "conflict", message: "docs/design/auth.md L42 — JWT 过期时间与代码实现不一致", time: "10 分钟前" },
  { id: "2", type: "gc", message: "progress.md 归档 8 条已完结事项 → 62 行", time: "32 分钟前" },
  { id: "3", type: "rule", message: "建议新增 Redis 分布式锁使用规范到 AGENTS.md", time: "1 小时前" },
  { id: "4", type: "prune", message: "识别到 3 段可被 ESLint 规则替代的文档描述", time: "2 小时前" },
];

const typeConfig = {
  gc: { color: "bg-green-100 text-green-700", label: "GC" },
  conflict: { color: "bg-red-100 text-red-700", label: "冲突" },
  rule: { color: "bg-blue-100 text-blue-700", label: "规则" },
  prune: { color: "bg-amber-100 text-amber-700", label: "清理" },
};

export function DashboardPage() {
  return (
    <div className="space-y-6">
      <h1 className="text-xl font-semibold">仪表盘</h1>

      <div className="grid grid-cols-6 gap-4">
        <HealthScore score={87} />
        <MetricCard label="文档总数" value={12} icon={FileText} color="blue" />
        <MetricCard label="活跃冲突" value={3} icon={AlertTriangle} color="red" />
        <MetricCard label="合规违规" value={0} icon={ShieldAlert} color="amber" />
        <MetricCard label="规则建议" value={1} icon={BookOpen} color="green" />
        <MetricCard label="过期文档" value={5} icon={Clock} color="blue" />
      </div>

      <div className="grid grid-cols-2 gap-4">
        {/* Recent Activity */}
        <div className="card p-5">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-sm font-semibold text-gray-700">最近活动</h2>
            <TrendingUp className="h-4 w-4 text-gray-400" />
          </div>
          <ul className="space-y-3">
            {mockActivities.map((a) => (
              <li key={a.id} className="flex items-start gap-3">
                <span className={cn("badge mt-0.5 shrink-0", typeConfig[a.type].color)}>
                  {typeConfig[a.type].label}
                </span>
                <div className="min-w-0 flex-1">
                  <p className="text-sm text-gray-700 leading-snug">{a.message}</p>
                  <p className="text-xs text-gray-400 mt-0.5">{a.time}</p>
                </div>
              </li>
            ))}
          </ul>
        </div>

        {/* Document Layer Distribution */}
        <div className="card p-5">
          <h2 className="mb-4 text-sm font-semibold text-gray-700">文档层级分布</h2>
          <div className="space-y-3">
            {[
              { label: "宪法与边界层", count: 2, total: 12, color: "bg-red-500" },
              { label: "状态与记忆层", count: 3, total: 12, color: "bg-amber-500" },
              { label: "契约与蓝图层", count: 5, total: 12, color: "bg-brand-500" },
              { label: "决策快照层", count: 2, total: 12, color: "bg-gray-400" },
            ].map((layer) => (
              <div key={layer.label}>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-600">{layer.label}</span>
                  <span className="font-medium tabular-nums">{layer.count}</span>
                </div>
                <div className="mt-1 h-2 rounded-full bg-gray-100">
                  <div
                    className={cn("h-2 rounded-full", layer.color)}
                    style={{ width: `${(layer.count / layer.total) * 100}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
