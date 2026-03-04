import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface UseTauriCommandResult<T> {
  data: T | null;
  error: string | null;
  isLoading: boolean;
  execute: (...args: unknown[]) => Promise<T | null>;
}

export function useTauriCommand<T>(command: string): UseTauriCommandResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (...args: unknown[]): Promise<T | null> => {
      setIsLoading(true);
      setError(null);
      try {
        // @ts-ignore
        if (typeof window !== 'undefined' && !window.__TAURI_INTERNALS__) {
             throw new Error("Tauri API 未就绪 (请在桌面客户端中运行)");
        }
        const result = await invoke<T>(command, args[0] as Record<string, unknown>);
        setData(result);
        return result;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [command],
  );

  return { data, error, isLoading, execute };
}
