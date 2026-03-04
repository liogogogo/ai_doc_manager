import { useState } from "react";
import { Recycle, Play, Archive, FileText } from "lucide-react";
import { cn } from "@/lib/utils";

interface GCTarget {
  path: string;
  currentLines: number;
  capacity: number;
  completedItems: number;
  status: "idle" | "scanning" | "done";
}

const mockTargets: GCTarget[] = [
  { path: ".ai/progress.md", currentLines: 142, capacity: 100, completedItems: 8, status: "idle" },
  { path: ".ai/memory.md", currentLines: 67, capacity: 100, completedItems: 2, status: "idle" },
];

export function GCPage() {
  const [targets, setTargets] = useState(mockTargets);

  const runGC = (index: number) => {
    setTargets((prev) =>
      prev.map((t, i) =>
        i === index ? { ...t, status: "scanning" as const } : t,
      ),
    );
    setTimeout(() => {
      setTargets((prev) =>
        prev.map((t, i) =>
          i === index
            ? { ...t, status: "done" as const, currentLines: t.currentLines - 40, completedItems: 0 }
            : t,
        ),
      );
    }, 2000);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">记忆垃圾回收</h1>
        <button className="btn-primary">
          <Play className="h-4 w-4" />
          全量回收
        </button>
      </div>

      <div className="space-y-3">
        {targets.map((target, index) => {
          const overCapacity = target.currentLines > target.capacity;
          return (
            <div key={target.path} className="card p-5">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <FileText className="h-5 w-5 text-gray-400" />
                  <div>
                    <p className="font-mono text-sm font-medium">{target.path}</p>
                    <p className="text-xs text-gray-500 mt-0.5">
                      {target.completedItems} 个已完结条目待归档
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-4">
                  <div className="text-right">
                    <p className={cn("text-sm font-medium tabular-nums", overCapacity ? "text-red-600" : "text-gray-700")}>
                      {target.currentLines} / {target.capacity} 行
                    </p>
                    <div className="mt-1 h-1.5 w-24 rounded-full bg-gray-100">
                      <div
                        className={cn(
                          "h-1.5 rounded-full transition-all",
                          overCapacity ? "bg-red-500" : "bg-green-500",
                        )}
                        style={{ width: `${Math.min((target.currentLines / target.capacity) * 100, 100)}%` }}
                      />
                    </div>
                  </div>
                  <button
                    onClick={() => runGC(index)}
                    disabled={target.status === "scanning"}
                    className="btn-secondary"
                  >
                    {target.status === "scanning" ? (
                      <Recycle className="h-4 w-4 animate-spin" />
                    ) : target.status === "done" ? (
                      <Archive className="h-4 w-4 text-green-600" />
                    ) : (
                      <Recycle className="h-4 w-4" />
                    )}
                    {target.status === "scanning" ? "回收中…" : target.status === "done" ? "已完成" : "执行回收"}
                  </button>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
