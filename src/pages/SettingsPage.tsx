import { useEffect, useState } from "react";
import { CheckCircle, Eye, EyeOff, Loader2, Save, TestTube, XCircle } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

interface LlmConfig {
  provider: string;
  base_url: string;
  model: string;
  api_key?: string;
  max_tokens_per_request: number;
}

const PROVIDER_PRESETS: Record<
  string,
  { label: string; url: string; model: string; needsKey: boolean; hint?: string }
> = {
  ollama:      { label: "Ollama（本地）",         url: "http://localhost:11434/v1",                             model: "llama3.1:8b",                 needsKey: false },
  openai:      { label: "OpenAI",                 url: "https://api.openai.com/v1",                            model: "gpt-4o-mini",                 needsKey: true  },
  deepseek:    { label: "DeepSeek",               url: "https://api.deepseek.com/v1",                          model: "deepseek-chat",               needsKey: true  },
  siliconflow: { label: "SiliconFlow（硅基流动）", url: "https://api.siliconflow.cn/v1",                        model: "Qwen/Qwen2.5-7B-Instruct",    needsKey: true  },
  qwen:        { label: "通义千问（阿里云）",      url: "https://dashscope.aliyuncs.com/compatible-mode/v1",   model: "qwen-turbo",                  needsKey: true  },
  zhipu:       { label: "智谱 AI（GLM）",          url: "https://open.bigmodel.cn/api/paas/v4",                 model: "glm-4-flash",                 needsKey: true  },
  moonshot:    { label: "Moonshot（Kimi）",        url: "https://api.moonshot.cn/v1",                           model: "moonshot-v1-8k",              needsKey: true  },
  spark:       {
    label: "讯飞星火",
    url: "https://spark-api-open.xf-yun.com/v1",
    model: "lite",
    needsKey: true,
    hint: "模型名可选：lite / generalv3 / pro-128k / generalv3.5 / 4.0Ultra。每个模型需在控制台单独「领取」或购买资源包，账户余额不等于资源包可用。",
  },
  claude:      { label: "Claude（Anthropic）",    url: "https://api.anthropic.com/v1",                         model: "claude-3-haiku-20240307",     needsKey: true  },
  custom:      { label: "自定义 API",              url: "",                                                      model: "",                            needsKey: true  },
};

/** 将 API 原始错误信息翻译为用户可读的操作建议 */
function parseErrorHint(raw: string): string {
  // 错误码 1113 — 讯飞星火 "余额不足或无可用资源包"
  if (raw.includes("1113") || raw.includes("余额不足") || raw.includes("无可用资源包")) {
    return '资源包未开通：账户余额 ≠ 模型资源包。请登录 console.xfyun.cn → 我的应用 → 对应模型 → 领取/购买资源包。';
  }
  // 401 / 403
  if (raw.includes("401") || raw.includes("Unauthorized") || raw.includes("authentication")) {
    return 'API Key 无效或已过期，请检查 Key 是否填写正确。';
  }
  if (raw.includes("403") || raw.includes("Forbidden") || raw.includes("permission")) {
    return 'API Key 权限不足，请确认该 Key 有调用此模型的权限。';
  }
  // 404
  if (raw.includes("404") || raw.includes("model_not_found") || raw.includes("does not exist")) {
    return '模型名称不存在，请检查「模型名称」是否填写正确（区分大小写）。';
  }
  // 429 rate limit
  if (raw.includes("429") || raw.includes("rate_limit") || raw.includes("Too Many")) {
    return '请求频率超限（Rate Limit）或配额耗尽，请稍后重试或升级套餐。';
  }
  // Connection refused
  if (raw.includes("Connection refused") || raw.includes("connect error")) {
    return '无法连接到服务器，请检查 API 地址是否正确，以及本地网络/代理设置。';
  }
  return raw;
}

type TestStatus = "idle" | "testing" | "ok" | "fail";

export function SettingsPage() {
  const [provider, setProvider] = useState("ollama");
  const [model, setModel] = useState("llama3.1:8b");
  const [baseUrl, setBaseUrl] = useState("http://localhost:11434/v1");
  const [apiKey, setApiKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [maxTokens, setMaxTokens] = useState(16384);
  const [gcInterval, setGcInterval] = useState(30);
  const [gcCapacity, setGcCapacity] = useState(100);
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [testStatus, setTestStatus] = useState<TestStatus>("idle");
  const [testMsg, setTestMsg] = useState("");

  // Load saved config on mount
  useEffect(() => {
    invoke<LlmConfig | null>("get_llm_config")
      .then((cfg) => {
        if (cfg) {
          setProvider(cfg.provider);
          setModel(cfg.model);
          setBaseUrl(cfg.base_url);
          setApiKey(cfg.api_key ?? "");
          setMaxTokens(cfg.max_tokens_per_request ?? 16384);
        }
      })
      .catch(() => {});
  }, []);

  function handleProviderChange(p: string) {
    setProvider(p);
    const preset = PROVIDER_PRESETS[p];
    if (preset) {
      if (preset.url) setBaseUrl(preset.url);
      if (preset.model) setModel(preset.model);
    }
  }

  async function handleSave() {
    setSaving(true);
    setSaveMsg(null);
    try {
      await invoke("save_llm_config", {
        config: {
          provider,
          base_url: baseUrl,
          model,
          api_key: apiKey || undefined,
          max_tokens_per_request: maxTokens,
        } satisfies LlmConfig,
      });
      setSaveMsg({ ok: true, text: "设置已保存" });
    } catch (e) {
      setSaveMsg({ ok: false, text: String(e) });
    } finally {
      setSaving(false);
    }
  }

  async function handleTest() {
    setTestStatus("testing");
    setTestMsg("");
    try {
      await invoke("test_llm_connection", {
        config: {
          provider,
          base_url: baseUrl,
          model,
          api_key: apiKey || undefined,
          max_tokens_per_request: maxTokens,
        } satisfies LlmConfig,
      });
      setTestStatus("ok");
      setTestMsg("连接成功");
    } catch (e) {
      setTestStatus("fail");
      setTestMsg(parseErrorHint(String(e)));
    }
  }

  const needsKey = PROVIDER_PRESETS[provider]?.needsKey ?? true;

  return (
    <div className="mx-auto max-w-2xl space-y-8">
      <h1 className="text-xl font-semibold">设置</h1>

      {/* LLM Configuration */}
      <section className="card p-6">
        <h2 className="mb-4 text-base font-semibold text-gray-900">LLM 配置</h2>
        <div className="space-y-4">

          {/* Provider */}
          <div>
            <label className="mb-1.5 block text-sm font-medium text-gray-700">模型提供商</label>
            <select
              value={provider}
              onChange={(e) => handleProviderChange(e.target.value)}
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
            >
              {Object.entries(PROVIDER_PRESETS).map(([val, { label }]) => (
                <option key={val} value={val}>{label}</option>
              ))}
            </select>
            {PROVIDER_PRESETS[provider]?.hint && (
              <p className="mt-1.5 rounded-md bg-amber-50 px-3 py-2 text-xs text-amber-700 leading-relaxed">
                ⚠ {PROVIDER_PRESETS[provider].hint}
              </p>
            )}
          </div>

          {/* Model */}
          <div>
            <label className="mb-1.5 block text-sm font-medium text-gray-700">模型名称</label>
            <input
              type="text"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
            />
          </div>

          {/* Base URL */}
          <div>
            <label className="mb-1.5 block text-sm font-medium text-gray-700">API 地址</label>
            <input
              type="text"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
            />
          </div>

          {/* API Key */}
          {needsKey && (
            <div>
              <label className="mb-1.5 block text-sm font-medium text-gray-700">API Key</label>
              <div className="relative">
                <input
                  type={showKey ? "text" : "password"}
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder="sk-..."
                  className="w-full rounded-lg border border-gray-300 px-3 py-2 pr-10 text-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
                />
                <button
                  type="button"
                  onClick={() => setShowKey((v) => !v)}
                  className="absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
                >
                  {showKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              </div>
              <p className="mt-1 text-xs text-gray-500">API Key 仅存储在本地，不会上传</p>
            </div>
          )}

          {/* Max Tokens */}
          <div>
            <label className="mb-1.5 block text-sm font-medium text-gray-700">
              单次最大 Token 数
            </label>
            <input
              type="number"
              value={maxTokens}
              onChange={(e) => setMaxTokens(Number(e.target.value))}
              min={512}
              max={128000}
              step={512}
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
            />
          </div>

          {/* Test Connection */}
          <div className="flex items-center gap-3">
            <button
              onClick={handleTest}
              disabled={testStatus === "testing"}
              className="btn-secondary"
            >
              {testStatus === "testing"
                ? <Loader2 className="h-4 w-4 animate-spin" />
                : <TestTube className="h-4 w-4" />}
              测试连接
            </button>
            {testStatus === "ok" && (
              <span className="flex items-center gap-1 text-sm text-green-600">
                <CheckCircle className="h-4 w-4" /> {testMsg}
              </span>
            )}
            {testStatus === "fail" && (
              <span className="flex items-start gap-1 text-sm text-red-500">
                <XCircle className="mt-0.5 h-4 w-4 shrink-0" />
                <span className="leading-snug">{testMsg}</span>
              </span>
            )}
          </div>
        </div>
      </section>

      {/* GC Configuration */}
      <section className="card p-6">
        <h2 className="mb-4 text-base font-semibold text-gray-900">记忆回收配置</h2>
        <div className="space-y-4">
          <div>
            <label className="mb-1.5 block text-sm font-medium text-gray-700">
              自动回收间隔（分钟）
            </label>
            <input
              type="number"
              value={gcInterval}
              onChange={(e) => setGcInterval(Number(e.target.value))}
              min={5}
              max={1440}
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
            />
          </div>

          <div>
            <label className="mb-1.5 block text-sm font-medium text-gray-700">
              状态文件容量上限（行）
            </label>
            <input
              type="number"
              value={gcCapacity}
              onChange={(e) => setGcCapacity(Number(e.target.value))}
              min={20}
              max={500}
              className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
            />
          </div>
        </div>
      </section>

      {/* Save */}
      <div className="flex items-center justify-end gap-3">
        {saveMsg && (
          <span className={`flex items-center gap-1 text-sm ${saveMsg.ok ? "text-green-600" : "text-red-500"}`}>
            {saveMsg.ok
              ? <CheckCircle className="h-4 w-4" />
              : <XCircle className="h-4 w-4" />}
            {saveMsg.text}
          </span>
        )}
        <button onClick={handleSave} disabled={saving} className="btn-primary">
          {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
          保存设置
        </button>
      </div>
    </div>
  );
}
