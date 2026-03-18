<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useTauri } from '@/composables/useTauri';
import FabricSelect from '@/components/ui/FabricSelect.vue';

const { invoke } = useTauri();

// Full config state (per-provider)
interface ProviderConfig {
  model: string;
  // empty string = auto / provider default
  dimensions: string;
  has_api_key: boolean;
  masked_api_key: string | null;
  base_url: string | null;
  proxy_url: string | null;
}

interface ProviderConfigResponse {
  model: string;
  dimensions: number | null;
  has_api_key: boolean;
  masked_api_key: string | null;
  base_url: string | null;
  proxy_url: string | null;
}

function toProviderConfig(resp: ProviderConfigResponse): ProviderConfig {
  return {
    model: resp.model ?? '',
    dimensions: resp.dimensions != null ? String(resp.dimensions) : '',
    has_api_key: resp.has_api_key ?? false,
    masked_api_key: resp.masked_api_key ?? null,
    base_url: resp.base_url ?? null,
    proxy_url: resp.proxy_url ?? null,
  };
}

const activeProvider = ref<string>('gemini');
const geminiConfig = ref<ProviderConfig>({
  model: '',
  dimensions: '',
  has_api_key: false,
  masked_api_key: null,
  base_url: null,
  proxy_url: null,
});
const openaiConfig = ref<ProviderConfig>({
  model: '',
  dimensions: '',
  has_api_key: false,
  masked_api_key: null,
  base_url: null,
  proxy_url: null,
});

// Current provider's editable fields
const apiKey = ref<string>('');

// UI state
const isLoading = ref(false);
const isSaving = ref(false);
const isTesting = ref(false);
const isRefreshingModels = ref(false);
const showPassword = ref(false);
const testResult = ref<{ success: boolean; message: string; latency_ms?: number; dimensions?: number } | null>(null);
const saveMessage = ref<string>('');
const refreshMessage = ref<string>('');

// Dynamic model list (empty by default, populated after refresh)
const availableModels = ref<Array<{ value: string; label: string }>>([]);

// Vector DB compatibility / rebuild prompt
interface VectorDbSignatureInfo {
  provider: string;
  model: string;
  dimensions: number;
  updated_at?: string | null;
}

interface VectorDbIncompatibilityInfo {
  book_id: string;
  title: string;
  chunk_count: number;
  embedding_count: number;
  db_signature: VectorDbSignatureInfo | null;
  reason: string;
}

interface VectorDbCompatibilityResult {
  current: { provider: string; model: string; dimensions: number | null };
  incompatible: VectorDbIncompatibilityInfo[];
  total_books: number;
}

const vectorDbReport = ref<VectorDbCompatibilityResult | null>(null);
const isCheckingVectorDb = ref(false);
const isRebuildingVectorDb = ref(false);
const rebuildProgress = ref<{ total: number; done: number; current?: string; errors: string[] } | null>(null);

// Provider options
const providerOptions = [
  { value: 'gemini', label: 'Google Gemini' },
  { value: 'openai', label: 'OpenAI' },
];

// Get current provider config
const currentConfig = computed(() => {
  return activeProvider.value === 'gemini' ? geminiConfig.value : openaiConfig.value;
});

// Computed: model options with placeholder if empty
const modelOptions = computed(() => {
  if (availableModels.value.length === 0) {
    return [{ value: '', label: '点击右侧刷新按钮获取模型列表' }];
  }
  return availableModels.value;
});

// Whether this provider supports proxy
const supportsProxy = computed(() => {
  return activeProvider.value === 'gemini';
});

// Whether this provider supports base URL
const supportsBaseUrl = computed(() => {
  return activeProvider.value === 'openai';
});

async function checkVectorDbCompatibility() {
  isCheckingVectorDb.value = true;
  try {
    vectorDbReport.value = await invoke<VectorDbCompatibilityResult>('check_vector_db_compatibility');
  } catch (e) {
    console.error('Failed to check vector DB compatibility:', e);
    vectorDbReport.value = null;
  } finally {
    isCheckingVectorDb.value = false;
  }
}

async function rebuildIncompatibleVectorDbs() {
  if (!vectorDbReport.value || vectorDbReport.value.incompatible.length === 0) return;

  isRebuildingVectorDb.value = true;
  rebuildProgress.value = {
    total: vectorDbReport.value.incompatible.length,
    done: 0,
    errors: [],
  };

  try {
    for (const item of vectorDbReport.value.incompatible) {
      rebuildProgress.value.current = item.title;
      try {
        await invoke('rebuild_book_embeddings', { bookId: item.book_id });
      } catch (e) {
        rebuildProgress.value.errors.push(`${item.title}: ${String(e)}`);
      } finally {
        rebuildProgress.value.done += 1;
      }
    }
  } finally {
    rebuildProgress.value.current = undefined;
    isRebuildingVectorDb.value = false;
    await checkVectorDbCompatibility();
  }
}

// Check if config is complete for current provider
const isConfigComplete = computed(() => {
  const config = currentConfig.value;
  const hasKey = config.has_api_key || apiKey.value.length > 0;

  if (activeProvider.value === 'gemini') {
    // Gemini: API key required, proxy optional
    return hasKey && config.model.length > 0;
  }
  // OpenAI: API key and base_url required
  const hasBaseUrl = config.base_url && config.base_url.length > 0;
  return hasKey && hasBaseUrl && config.model.length > 0;
});

// Check what's missing for refresh
const refreshRequirements = computed(() => {
  const config = currentConfig.value;
  const missing: string[] = [];

  if (!config.has_api_key && !apiKey.value) {
    missing.push('API Key');
  }

  if (activeProvider.value === 'openai') {
    if (!config.base_url) {
      missing.push('Base URL');
    }
  }

  return missing;
});

// Refresh model list from API
async function refreshModels() {
  refreshMessage.value = '';

  // Check if config is complete
  if (refreshRequirements.value.length > 0) {
    refreshMessage.value = `请先配置 ${refreshRequirements.value.join(' 和 ')}`;
    return;
  }

  isRefreshingModels.value = true;
  const config = currentConfig.value;

  try {
    const result = await invoke<{
      success: boolean;
      models: Array<{ id: string; name: string }>;
      message: string;
    }>('fetch_embedding_models', {
      provider: activeProvider.value,
      apiKey: apiKey.value || null,
      apiKeyRef: config.has_api_key ? `embedding_${activeProvider.value}` : null,
      baseUrl: config.base_url || null,
      proxyUrl: config.proxy_url || null,
    });

    if (result.success) {
      availableModels.value = result.models.map(m => ({
        value: m.id,
        label: m.name,
      }));

      // Auto-select first model if none selected
      if (result.models.length > 0 && !config.model) {
        if (activeProvider.value === 'gemini') {
          geminiConfig.value.model = result.models[0].id;
        } else {
          openaiConfig.value.model = result.models[0].id;
        }
      }

      refreshMessage.value = `获取到 ${result.models.length} 个模型`;
    } else {
      refreshMessage.value = result.message;
    }
  } catch (e) {
    console.error('Failed to fetch models:', e);
    refreshMessage.value = `获取失败: ${e}`;
  } finally {
    isRefreshingModels.value = false;

    // Clear message after 5 seconds
    setTimeout(() => {
      refreshMessage.value = '';
    }, 5000);
  }
}

// Load current config
async function loadConfig() {
  isLoading.value = true;
  try {
    const response = await invoke<{
      active_provider: string;
      gemini: ProviderConfigResponse;
      openai: ProviderConfigResponse;
    }>('get_embedding_config');

    activeProvider.value = response.active_provider === 'disabled' ? 'gemini' : response.active_provider;
    geminiConfig.value = toProviderConfig(response.gemini);
    openaiConfig.value = toProviderConfig(response.openai);

    // Update available models for current provider
    updateAvailableModels();

    // Check vectors.db signature compatibility for rebuild prompt
    await checkVectorDbCompatibility();
  } catch (e) {
    console.error('Failed to load embedding config:', e);
  } finally {
    isLoading.value = false;
  }
}

// Update available models based on current provider's saved model
function updateAvailableModels() {
  const config = currentConfig.value;
  if (config.model) {
    availableModels.value = [{ value: config.model, label: config.model }];
  } else {
    availableModels.value = [];
  }
}

function parseDimensions(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const n = Number(trimmed);
  if (!Number.isFinite(n) || !Number.isInteger(n) || n <= 0) return null;
  return n;
}

// Save config
async function saveConfig() {
  isSaving.value = true;
  saveMessage.value = '';
  testResult.value = null;

  const config = currentConfig.value;
  const dims = parseDimensions(config.dimensions);
  if (config.dimensions.trim() && dims === null) {
    saveMessage.value = 'Dimensions 必须是正整数（或留空表示默认）';
    isSaving.value = false;
    return;
  }

  try {
    await invoke('save_embedding_config', {
      request: {
        provider: activeProvider.value,
        model: config.model,
        dimensions: dims,
        api_key: apiKey.value || null,
        base_url: activeProvider.value === 'openai' ? (config.base_url || null) : null,
        proxy_url: activeProvider.value === 'gemini' ? (config.proxy_url || null) : null,
      },
    });

    saveMessage.value = '保存成功';
    apiKey.value = ''; // Clear input after save

    // Reload to get updated masked key
    await loadConfig();

    setTimeout(() => {
      saveMessage.value = '';
    }, 3000);
  } catch (e) {
    console.error('Failed to save embedding config:', e);
    saveMessage.value = `保存失败: ${e}`;
  } finally {
    isSaving.value = false;
  }
}

// Test connection
async function testConnection() {
  isTesting.value = true;
  testResult.value = null;

  try {
    // Save first if there are changes
    if (apiKey.value) {
      await saveConfig();
    }

    const result = await invoke<{
      success: boolean;
      message: string;
      latency_ms: number | null;
      dimensions?: number | null;
    }>('test_embedding_connection');

    testResult.value = {
      success: result.success,
      message: result.message,
      latency_ms: result.latency_ms ?? undefined,
      dimensions: result.dimensions ?? undefined,
    };
  } catch (e) {
    console.error('Failed to test embedding connection:', e);
    testResult.value = {
      success: false,
      message: `测试失败: ${e}`,
    };
  } finally {
    isTesting.value = false;
  }
}

// Watch provider changes to load that provider's config
watch(activeProvider, () => {
  // Reset UI state
  apiKey.value = '';
  refreshMessage.value = '';
  testResult.value = null;

  // Update available models for the new provider
  updateAvailableModels();
});

onMounted(() => {
  loadConfig();
});
</script>

<template>
  <div class="space-y-6">
    <!-- Configuration Status Banner -->
    <div
      v-if="!isConfigComplete"
      class="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-4"
    >
      <div class="flex items-start gap-3">
        <svg class="w-5 h-5 text-amber-500 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <div class="text-sm">
          <p class="font-medium text-amber-800 dark:text-amber-200">Embedding 服务未配置</p>
          <p class="text-amber-700 dark:text-amber-300 mt-1">
            导入书籍前需要先完成 Embedding 配置，用于生成文本向量和智能搜索。
          </p>
        </div>
      </div>
    </div>

    <!-- Vector DB Rebuild Prompt -->
    <div
      v-if="vectorDbReport && vectorDbReport.incompatible.length > 0"
      class="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-4"
    >
      <div class="flex items-start gap-3">
        <svg class="w-5 h-5 text-amber-500 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <div class="text-sm flex-1">
          <p class="font-medium text-amber-800 dark:text-amber-200">检测到向量库需要重建</p>
          <p class="text-amber-700 dark:text-amber-300 mt-1">
            当前 Embedding 设置与以下 {{ vectorDbReport.incompatible.length }} 本书的向量库不一致，语义搜索可能失效。
          </p>

          <div class="mt-2 space-y-1">
            <div
              v-for="item in vectorDbReport.incompatible.slice(0, 5)"
              :key="item.book_id"
              class="text-xs text-amber-800/80 dark:text-amber-200/80"
            >
              {{ item.title }}（{{ item.reason }}）
            </div>
            <div
              v-if="vectorDbReport.incompatible.length > 5"
              class="text-xs text-amber-700/70 dark:text-amber-300/70"
            >
              还有 {{ vectorDbReport.incompatible.length - 5 }} 本书未显示……
            </div>
          </div>

          <div class="mt-3 flex items-center gap-3">
            <button
              @click="rebuildIncompatibleVectorDbs"
              :disabled="isRebuildingVectorDb || isLoading || !isConfigComplete"
              class="fabric-btn-secondary inline-flex items-center gap-2"
              :title="!isConfigComplete ? '请先完成 Embedding 配置' : '重建向量库'"
            >
              <svg v-if="isRebuildingVectorDb" class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              <span>重建向量库</span>
            </button>

            <button
              @click="checkVectorDbCompatibility"
              :disabled="isCheckingVectorDb || isLoading"
              class="text-xs text-amber-800/80 dark:text-amber-200/80 hover:underline disabled:opacity-50 disabled:cursor-not-allowed"
              title="重新检查"
            >
              {{ isCheckingVectorDb ? '检查中…' : '重新检查' }}
            </button>

            <div v-if="rebuildProgress" class="text-xs text-amber-700/70 dark:text-amber-300/70">
              {{ rebuildProgress.done }}/{{ rebuildProgress.total }}
              <span v-if="rebuildProgress.current">（正在重建: {{ rebuildProgress.current }}）</span>
            </div>
          </div>

          <div
            v-if="rebuildProgress && rebuildProgress.errors.length > 0"
            class="mt-2 text-xs text-red-600 dark:text-red-400 whitespace-pre-line"
          >
            {{ rebuildProgress.errors.join('\n') }}
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="isConfigComplete"
      class="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-4"
    >
      <div class="flex items-start gap-3">
        <svg class="w-5 h-5 text-green-500 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        </svg>
        <div class="text-sm">
          <p class="font-medium text-green-800 dark:text-green-200">Embedding 服务已配置</p>
          <p class="text-green-700 dark:text-green-300 mt-1">
            当前使用: {{ providerOptions.find(p => p.value === activeProvider)?.label || activeProvider }}
          </p>
        </div>
      </div>
    </div>

    <!-- Provider Selection -->
    <div>
      <label class="block text-sm font-medium text-fabric-sepia mb-1">Embedding 服务</label>
      <p class="text-xs text-fabric-thread/60 mb-2">
        选择用于生成文本向量的服务。向量用于智能上下文检索和语义搜索。
      </p>
      <FabricSelect
        v-model="activeProvider"
        :options="providerOptions"
        :disabled="isLoading"
      />
    </div>

    <!-- API Key -->
    <div>
      <label class="block text-sm font-medium text-fabric-sepia mb-1">API Key</label>
      <div class="relative">
        <input
          v-model="apiKey"
          :type="showPassword ? 'text' : 'password'"
          autocomplete="off"
          class="w-full px-3 py-2 pr-10 border border-fabric-sand/50 rounded-lg bg-fabric-linen text-fabric-sepia text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30 hide-password-toggle"
          :placeholder="currentConfig.has_api_key ? (currentConfig.masked_api_key || '已配置') : '输入 API Key'"
        />
        <button
          type="button"
          @click="showPassword = !showPassword"
          class="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 text-fabric-thread/50 hover:text-fabric-thread transition-colors rounded"
          :title="showPassword ? '隐藏' : '显示'"
        >
          <svg v-if="showPassword" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
          </svg>
          <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
          </svg>
        </button>
      </div>
      <p v-if="currentConfig.has_api_key" class="text-xs text-fabric-thread/60 mt-1">
        已配置 API Key。留空保持不变，或输入新值更新。
      </p>
    </div>

    <!-- Base URL (for OpenAI) -->
    <div v-if="supportsBaseUrl">
      <label class="block text-sm font-medium text-fabric-sepia mb-1">Base URL</label>
      <input
        v-model="openaiConfig.base_url"
        type="text"
        class="w-full px-3 py-2 border border-fabric-sand/50 rounded-lg bg-fabric-linen text-fabric-sepia text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30"
        placeholder="https://api.openai.com/v1"
      />
      <p class="text-xs text-fabric-thread/60 mt-1">
        使用兼容 OpenAI API 的 Embedding 服务时填写。
      </p>
    </div>

    <!-- Proxy URL (for Gemini) -->
    <div v-if="supportsProxy">
      <label class="block text-sm font-medium text-fabric-sepia mb-1">代理地址（可选）</label>
      <input
        v-model="geminiConfig.proxy_url"
        type="text"
        class="w-full px-3 py-2 border border-fabric-sand/50 rounded-lg bg-fabric-linen text-fabric-sepia text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30"
        placeholder="http://127.0.0.1:7890"
      />
      <p class="text-xs text-fabric-thread/60 mt-1">
        如果需要通过代理访问 Google API，请填写代理地址。
      </p>
    </div>

    <!-- Model Selection with Refresh Button -->
    <div>
      <div class="flex items-center justify-between mb-1">
        <label class="text-sm font-medium text-fabric-sepia">模型</label>
        <button
          @click="refreshModels"
          :disabled="isRefreshingModels || isLoading"
          class="flex items-center gap-1.5 px-2 py-1 text-xs text-fabric-thread/70 hover:text-primary-600 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-md transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
          title="从 API 获取模型列表"
        >
          <svg
            class="w-3.5 h-3.5 transition-transform duration-300"
            :class="{ 'animate-spin': isRefreshingModels }"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          <span>刷新</span>
        </button>
      </div>
      <!-- Gemini Model -->
      <FabricSelect
        v-if="activeProvider === 'gemini'"
        v-model="geminiConfig.model"
        :options="modelOptions"
        :disabled="isLoading || availableModels.length === 0"
      />
      <!-- OpenAI Model -->
      <FabricSelect
        v-else
        v-model="openaiConfig.model"
        :options="modelOptions"
        :disabled="isLoading || availableModels.length === 0"
      />
      <p v-if="refreshMessage" class="text-xs mt-1" :class="refreshMessage.includes('获取到') ? 'text-green-600' : 'text-amber-600'">
        {{ refreshMessage }}
      </p>
      <p v-else-if="availableModels.length === 0" class="text-xs text-fabric-thread/50 mt-1">
        {{ refreshRequirements.length > 0
          ? `请先配置 ${refreshRequirements.join(' 和 ')}，然后点击刷新获取模型列表`
          : '点击刷新按钮从 API 获取可用模型' }}
      </p>
    </div>

    <!-- Dimensions (optional) -->
    <div>
      <label class="block text-sm font-medium text-fabric-sepia mb-1">维度（Dimensions，可选）</label>

      <input
        v-if="activeProvider === 'gemini'"
        v-model="geminiConfig.dimensions"
        type="text"
        inputmode="numeric"
        class="w-full px-3 py-2 border border-fabric-sand/50 rounded-lg bg-fabric-linen text-fabric-sepia text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30"
        placeholder="留空=模型默认（Gemini 通常为 768）"
      />
      <input
        v-else
        v-model="openaiConfig.dimensions"
        type="text"
        inputmode="numeric"
        class="w-full px-3 py-2 border border-fabric-sand/50 rounded-lg bg-fabric-linen text-fabric-sepia text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30"
        placeholder="留空=模型默认（例如 1536 / 3072 / 1024）"
      />

      <p class="text-xs text-fabric-thread/60 mt-1">
        留空表示使用模型默认维度。修改模型或维度后，通常需要重建向量库（vectors.db），否则语义搜索可能失效。
      </p>
    </div>

    <!-- Actions -->
    <div class="space-y-3 pt-2">
      <div class="flex items-center gap-3">
        <button
          @click="saveConfig"
          :disabled="isSaving || isLoading"
          class="fabric-btn"
        >
          <svg v-if="isSaving" class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          <span v-else>保存配置</span>
        </button>

        <button
          @click="testConnection"
          :disabled="isTesting || isLoading || !isConfigComplete"
          class="fabric-btn-secondary inline-flex items-center gap-2"
          :title="!isConfigComplete ? '请先完成配置' : '测试连接'"
        >
          <svg v-if="isTesting" class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          <template v-else>
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            <span>测试连接</span>
          </template>
        </button>
      </div>

      <!-- Save message -->
      <transition
        enter-active-class="transition-all duration-300 ease-out"
        enter-from-class="opacity-0 -translate-y-1"
        enter-to-class="opacity-100 translate-y-0"
        leave-active-class="transition-all duration-200 ease-in"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div
          v-if="saveMessage"
          class="inline-flex items-center gap-1.5 text-sm px-3 py-2 rounded-md"
          :class="saveMessage.includes('失败')
            ? 'text-red-600 bg-red-50 dark:bg-red-900/20'
            : 'text-green-600 bg-green-50 dark:bg-green-900/20'"
        >
          <svg v-if="!saveMessage.includes('失败')" class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
          <svg v-else class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span>{{ saveMessage }}</span>
        </div>
      </transition>
    </div>

    <!-- Test result card -->
    <transition
      enter-active-class="transition-all duration-300 ease-out"
      enter-from-class="opacity-0 -translate-y-2"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition-all duration-200 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="testResult"
        class="rounded-xl overflow-hidden shadow-sm"
        :class="testResult.success
          ? 'bg-gradient-to-br from-green-50 to-emerald-50 dark:from-green-900/30 dark:to-emerald-900/20 border border-green-200/60 dark:border-green-700/40'
          : 'bg-gradient-to-br from-red-50 to-rose-50 dark:from-red-900/30 dark:to-rose-900/20 border border-red-200/60 dark:border-red-700/40'"
      >
        <div class="p-4">
          <div class="flex items-start gap-3">
            <!-- Status Icon -->
            <div
              class="shrink-0 w-10 h-10 rounded-full flex items-center justify-center"
              :class="testResult.success
                ? 'bg-green-100 dark:bg-green-800/40'
                : 'bg-red-100 dark:bg-red-800/40'"
            >
              <svg
                v-if="testResult.success"
                class="w-5 h-5 text-green-600 dark:text-green-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
              </svg>
              <svg
                v-else
                class="w-5 h-5 text-red-600 dark:text-red-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </div>

            <!-- Content -->
            <div class="flex-1 min-w-0">
              <h4
                class="font-medium text-sm"
                :class="testResult.success
                  ? 'text-green-800 dark:text-green-200'
                  : 'text-red-800 dark:text-red-200'"
              >
                {{ testResult.success ? '连接成功' : '连接失败' }}
              </h4>
              <p
                class="text-sm mt-0.5"
                :class="testResult.success
                  ? 'text-green-700/80 dark:text-green-300/80'
                  : 'text-red-700/80 dark:text-red-300/80'"
              >
                {{ testResult.message }}
              </p>
            </div>

            <!-- Latency Badge -->
            <div
              v-if="testResult.latency_ms"
              class="shrink-0 px-2.5 py-1 rounded-full text-xs font-medium"
              :class="testResult.success
                ? 'bg-green-100 dark:bg-green-800/50 text-green-700 dark:text-green-300'
                : 'bg-red-100 dark:bg-red-800/50 text-red-700 dark:text-red-300'"
            >
              <span class="inline-flex items-center gap-1">
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                {{ testResult.latency_ms }}ms
              </span>
            </div>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
/* Hide browser's default password toggle button */
.hide-password-toggle::-ms-reveal,
.hide-password-toggle::-ms-clear,
.hide-password-toggle::-webkit-credentials-auto-fill-button {
  display: none !important;
}

/* For Edge and Chrome */
input[type="password"]::-ms-reveal,
input[type="password"]::-ms-clear {
  display: none;
}

/* Webkit browsers (Chrome, Safari, Edge) */
input::-webkit-contacts-auto-fill-button,
input::-webkit-credentials-auto-fill-button {
  visibility: hidden;
  display: none !important;
  pointer-events: none;
  height: 0;
  width: 0;
  margin: 0;
}
</style>
