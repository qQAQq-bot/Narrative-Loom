<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import type { ProviderWithStatus, SaveProviderRequest } from '@/stores/settings';
import { useTauri } from '@/composables/useTauri';
import FabricSelect from '@/components/ui/FabricSelect.vue';

const { invoke } = useTauri();

const props = defineProps<{
  visible: boolean;
  provider?: ProviderWithStatus | null;
  isNew?: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', data: SaveProviderRequest): void;
  (e: 'delete', id: string): void;
}>();

// Form data
const formData = ref<SaveProviderRequest>({
  id: '',
  name: '',
  enabled: true,
  base_url: '',
  api_format: 'openai',
  path_override: null,
  api_key_ref: '',
  default_model: '',
  available_models: [],
  headers: {},
  timeout_ms: 60000,
  max_retries: 3,
  api_key: '',
});

const newModel = ref('');
const showApiKey = ref(false);
const isFetchingModels = ref(false);
const fetchModelsError = ref<string | null>(null);

// Initialize form when props change
watch(
  () => [props.visible, props.provider],
  () => {
    if (props.visible) {
      // Reset show password state
      showApiKey.value = false;

      if (props.provider && !props.isNew) {
        formData.value = {
          id: props.provider.id,
          name: props.provider.name,
          enabled: props.provider.enabled,
          base_url: props.provider.base_url,
          api_format: props.provider.api_format,
          path_override: props.provider.path_override,
          api_key_ref: props.provider.api_key_ref,
          default_model: props.provider.default_model,
          available_models: [...props.provider.available_models],
          headers: { ...props.provider.headers },
          timeout_ms: props.provider.timeout_ms,
          max_retries: props.provider.max_retries,
          api_key: '',
        };
      } else {
        // New provider
        formData.value = {
          id: '',
          name: '',
          enabled: true,
          base_url: 'https://api.openai.com',
          api_format: 'chat_completions',
          path_override: null,
          api_key_ref: '',
          default_model: 'gpt-4o-mini',
          available_models: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo'],
          headers: {},
          timeout_ms: 60000,
          max_retries: 3,
          api_key: '',
        };
      }
      newModel.value = '';
    }
  },
  { immediate: true }
);

// Auto-generate id from name
watch(
  () => formData.value.name,
  (name) => {
    if (props.isNew && name) {
      formData.value.id = name.toLowerCase().replace(/[^a-z0-9]+/g, '-');
      formData.value.api_key_ref = `${formData.value.id}_api_key`;
    }
  }
);

const title = computed(() => (props.isNew ? '添加 Provider' : '编辑 Provider'));

const apiFormatOptions = [
  { value: 'chat_completions', label: 'OpenAI Chat Completions (/v1/chat/completions)' },
  { value: 'responses', label: 'OpenAI Responses (/v1/responses)' },
  { value: 'anthropic', label: 'Anthropic (Claude)' },
  { value: 'ollama', label: 'Ollama (本地模型)' },
];

const presetProviders = [
  {
    name: 'OpenAI',
    base_url: 'https://api.openai.com',
    api_format: 'chat_completions' as const,
    models: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-3.5-turbo'],
  },
  {
    name: 'Anthropic (Claude)',
    base_url: 'https://api.anthropic.com',
    api_format: 'anthropic' as const,
    models: ['claude-sonnet-4-20250514', 'claude-opus-4-20250514', 'claude-3-5-sonnet-20241022', 'claude-3-5-haiku-20241022'],
  },
  {
    name: 'Ollama (本地)',
    base_url: 'http://localhost:11434',
    api_format: 'ollama' as const,
    models: ['qwen2.5:32b', 'qwen2.5:14b', 'llama3.1:70b', 'mistral:7b'],
  },
  {
    name: 'OpenRouter',
    base_url: 'https://openrouter.ai/api',
    api_format: 'chat_completions' as const,
    models: ['openai/gpt-4o', 'anthropic/claude-3-opus'],
  },
];

// Check if the selected format needs API key
const needsApiKey = computed(() => {
  return formData.value.api_format !== 'ollama';
});

function applyPreset(preset: (typeof presetProviders)[0]) {
  formData.value.name = preset.name;
  formData.value.base_url = preset.base_url;
  formData.value.api_format = preset.api_format;
  formData.value.available_models = [...preset.models];
  formData.value.default_model = preset.models[0] || '';
}

function addModel() {
  const model = newModel.value.trim();
  if (model && !formData.value.available_models.includes(model)) {
    formData.value.available_models.push(model);
    newModel.value = '';
  }
}

function removeModel(index: number) {
  formData.value.available_models.splice(index, 1);
}

function toggleShowApiKey() {
  showApiKey.value = !showApiKey.value;
}

// Fetch models from API
async function fetchModels() {
  if (!formData.value.base_url) {
    fetchModelsError.value = '请先填写 API 地址';
    return;
  }

  isFetchingModels.value = true;
  fetchModelsError.value = null;

  try {
    const result = await invoke<{ success: boolean; models: string[]; message: string }>(
      'fetch_provider_models',
      {
        baseUrl: formData.value.base_url,
        apiFormat: formData.value.api_format,
        apiKey: formData.value.api_key || null,
        apiKeyRef: formData.value.api_key_ref || null,
      }
    );

    if (result.success && result.models.length > 0) {
      formData.value.available_models = result.models;
      if (!formData.value.default_model || !result.models.includes(formData.value.default_model)) {
        formData.value.default_model = result.models[0];
      }
    } else {
      fetchModelsError.value = result.message || '未获取到模型列表';
    }
  } catch (e) {
    fetchModelsError.value = `获取失败: ${e}`;
  } finally {
    isFetchingModels.value = false;
  }
}

function handleSave() {
  emit('save', { ...formData.value });
}

function handleDelete() {
  if (props.provider) {
    emit('delete', props.provider.id);
  }
}

function handleClose() {
  emit('close');
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="visible"
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
        @click.self="handleClose"
      >
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-fabric-sepia/30 dark:bg-black/50 backdrop-blur-sm"></div>

        <!-- Modal -->
        <div
          class="relative bg-fabric-cream dark:bg-fabric-canvas rounded-2xl shadow-fabric-lg w-full max-w-2xl max-h-[90vh] flex flex-col overflow-hidden border border-fabric-sand/50 dark:border-fabric-sand/30"
        >
          <!-- Header -->
          <div class="flex items-center justify-between px-6 py-4 border-b border-fabric-sand/40 bg-fabric-linen/50">
            <h3 class="text-lg font-bold text-fabric-sepia font-serif">{{ title }}</h3>
            <button
              @click="handleClose"
              class="p-2 text-fabric-thread/60 hover:text-fabric-sepia hover:bg-fabric-sand/30 rounded-lg transition-colors duration-180"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                class="h-5 w-5"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fill-rule="evenodd"
                  d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                  clip-rule="evenodd"
                />
              </svg>
            </button>
          </div>

          <!-- Content -->
          <div class="flex-1 overflow-y-auto px-6 py-4 bg-fabric-warm">
            <!-- Presets -->
            <div v-if="isNew" class="mb-6">
              <label class="block text-sm font-medium text-fabric-sepia mb-2">快速选择</label>
              <div class="flex flex-wrap gap-2">
                <button
                  v-for="preset in presetProviders"
                  :key="preset.name"
                  @click="applyPreset(preset)"
                  class="px-3 py-1.5 text-sm bg-fabric-linen hover:bg-primary-500/10 hover:text-primary-600 text-fabric-thread rounded-lg transition-colors duration-180 border border-fabric-sand/40"
                >
                  {{ preset.name }}
                </button>
              </div>
            </div>

            <div class="space-y-4">
              <!-- Name & ID -->
              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label class="block text-sm font-medium text-fabric-sepia mb-1">名称 *</label>
                  <input
                    v-model="formData.name"
                    type="text"
                    class="fabric-input"
                    placeholder="OpenAI"
                  />
                </div>
                <div>
                  <label class="block text-sm font-medium text-fabric-sepia mb-1">ID</label>
                  <input
                    v-model="formData.id"
                    type="text"
                    :disabled="!isNew"
                    class="fabric-input disabled:bg-fabric-linen/50 disabled:text-fabric-thread/50"
                    placeholder="openai"
                  />
                </div>
              </div>

              <!-- Base URL -->
              <div>
                <label class="block text-sm font-medium text-fabric-sepia mb-1">API 地址 *</label>
                <input
                  v-model="formData.base_url"
                  type="url"
                  class="fabric-input font-mono text-sm"
                  placeholder="https://api.openai.com"
                />
              </div>

              <!-- API Format -->
              <div>
                <label class="block text-sm font-medium text-fabric-sepia mb-1">API 格式</label>
                <FabricSelect
                  v-model="formData.api_format"
                  :options="apiFormatOptions"
                />
              </div>

              <!-- API Key -->
              <div v-if="needsApiKey">
                <label class="block text-sm font-medium text-fabric-sepia mb-1">
                  API Key
                  <span v-if="!isNew && provider?.has_api_key" class="inline-flex items-center gap-1 text-accent-character text-xs ml-2">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" viewBox="0 0 20 20" fill="currentColor">
                      <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
                    </svg>
                    已保存: {{ provider?.masked_api_key }}
                  </span>
                </label>
                <div class="relative">
                  <input
                    v-model="formData.api_key"
                    :type="showApiKey ? 'text' : 'password'"
                    class="fabric-input font-mono text-sm pr-10"
                    :placeholder="isNew ? 'sk-...' : '留空保持不变'"
                    autocomplete="off"
                  />
                  <button
                    type="button"
                    @click="toggleShowApiKey"
                    class="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-fabric-thread/50 hover:text-fabric-sepia transition-colors duration-180"
                    :title="showApiKey ? '隐藏 API Key' : '显示 API Key'"
                  >
                    <!-- Eye icon (show) -->
                    <svg v-if="!showApiKey" xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                      <path d="M10 12a2 2 0 100-4 2 2 0 000 4z" />
                      <path fill-rule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clip-rule="evenodd" />
                    </svg>
                    <!-- Eye-off icon (hide) -->
                    <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                      <path fill-rule="evenodd" d="M3.707 2.293a1 1 0 00-1.414 1.414l14 14a1 1 0 001.414-1.414l-1.473-1.473A10.014 10.014 0 0019.542 10C18.268 5.943 14.478 3 10 3a9.958 9.958 0 00-4.512 1.074l-1.78-1.781zm4.261 4.26l1.514 1.515a2.003 2.003 0 012.45 2.45l1.514 1.514a4 4 0 00-5.478-5.478z" clip-rule="evenodd" />
                      <path d="M12.454 16.697L9.75 13.992a4 4 0 01-3.742-3.741L2.335 6.578A9.98 9.98 0 00.458 10c1.274 4.057 5.065 7 9.542 7 .847 0 1.669-.105 2.454-.303z" />
                    </svg>
                  </button>
                </div>
                <p class="text-xs text-fabric-thread/60 mt-1 flex items-center gap-1">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" viewBox="0 0 20 20" fill="currentColor">
                    <path fill-rule="evenodd" d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z" clip-rule="evenodd" />
                  </svg>
                  API Key 将安全存储在系统密钥链中
                </p>
              </div>

              <!-- No API Key needed notice for Ollama -->
              <div v-else class="bg-primary-500/10 border border-primary-400/30 rounded-lg p-3">
                <p class="text-sm text-primary-700 flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
                    <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd" />
                  </svg>
                  Ollama 本地模型不需要 API Key
                </p>
              </div>

              <!-- Default Model -->
              <div>
                <div class="flex items-center justify-between mb-1">
                  <label class="text-sm font-medium text-fabric-sepia">默认模型</label>
                  <button
                    type="button"
                    @click="fetchModels"
                    :disabled="isFetchingModels"
                    class="flex items-center gap-1 text-xs text-primary-600 hover:text-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors duration-180"
                    title="从 API 获取模型列表"
                  >
                    <svg
                      :class="['w-3.5 h-3.5', isFetchingModels ? 'animate-spin' : '']"
                      xmlns="http://www.w3.org/2000/svg"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                    </svg>
                    {{ isFetchingModels ? '获取中...' : '刷新' }}
                  </button>
                </div>
                <FabricSelect
                  v-model="formData.default_model"
                  :options="formData.available_models.map(m => ({ value: m, label: m }))"
                  placeholder="选择默认模型"
                  searchable
                  search-placeholder="搜索模型..."
                />
                <p v-if="fetchModelsError" class="text-xs text-red-500 mt-1">{{ fetchModelsError }}</p>
              </div>

              <!-- Available Models -->
              <div>
                <label class="block text-sm font-medium text-fabric-sepia mb-1">可用模型</label>
                <div class="flex gap-2 mb-2">
                  <input
                    v-model="newModel"
                    type="text"
                    class="fabric-input flex-1"
                    placeholder="添加模型名称"
                    @keyup.enter="addModel"
                  />
                  <button
                    @click="addModel"
                    class="px-3 py-2 bg-fabric-linen hover:bg-fabric-sand/40 text-fabric-sepia rounded-lg transition-colors duration-180 border border-fabric-sand/40"
                  >
                    添加
                  </button>
                </div>
                <div class="flex flex-wrap gap-2">
                  <span
                    v-for="(model, index) in formData.available_models"
                    :key="model"
                    class="inline-flex items-center gap-1 px-2 py-1 bg-fabric-linen rounded-lg text-sm font-mono text-fabric-thread border border-fabric-sand/30"
                  >
                    {{ model }}
                    <button
                      @click="removeModel(index)"
                      class="text-fabric-thread/50 hover:text-red-500 transition-colors duration-180"
                    >
                      ×
                    </button>
                  </span>
                </div>
              </div>

              <!-- Advanced Settings -->
              <details class="border border-fabric-sand/40 rounded-lg bg-fabric-linen/30">
                <summary class="px-4 py-2 cursor-pointer text-sm font-medium text-fabric-sepia hover:bg-fabric-sand/20 rounded-lg transition-colors duration-180">
                  高级设置
                </summary>
                <div class="px-4 py-3 border-t border-fabric-sand/30 space-y-4">
                  <div class="grid grid-cols-2 gap-4">
                    <div>
                      <label class="block text-sm font-medium text-fabric-sepia mb-1">
                        超时时间 (ms)
                      </label>
                      <input
                        v-model.number="formData.timeout_ms"
                        type="number"
                        class="fabric-input"
                      />
                    </div>
                    <div>
                      <label class="block text-sm font-medium text-fabric-sepia mb-1">
                        最大重试次数
                      </label>
                      <input
                        v-model.number="formData.max_retries"
                        type="number"
                        min="0"
                        max="10"
                        class="fabric-input"
                      />
                    </div>
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-fabric-sepia mb-1">
                      路径覆盖 (可选)
                    </label>
                    <input
                      v-model="formData.path_override"
                      type="text"
                      class="fabric-input font-mono text-sm"
                      placeholder="/v1/chat/completions"
                    />
                  </div>
                </div>
              </details>

              <!-- Enabled Toggle -->
              <div class="flex items-center gap-3">
                <input
                  v-model="formData.enabled"
                  type="checkbox"
                  id="provider-enabled"
                  class="w-4 h-4 text-primary-600 border-fabric-sand rounded focus:ring-primary-500 bg-fabric-warm"
                />
                <label for="provider-enabled" class="text-sm font-medium text-fabric-sepia">
                  启用此 Provider
                </label>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-between px-6 py-4 border-t border-fabric-sand/40 bg-fabric-linen/50">
            <div>
              <button
                v-if="!isNew"
                @click="handleDelete"
                class="px-4 py-2 text-red-600 hover:bg-red-50 rounded-lg transition-colors duration-180 text-sm font-medium"
              >
                删除
              </button>
            </div>
            <div class="flex gap-3">
              <button
                @click="handleClose"
                class="px-4 py-2 text-fabric-thread/70 hover:bg-fabric-sand/30 rounded-lg transition-colors duration-180 text-sm font-medium"
              >
                取消
              </button>
              <button
                @click="handleSave"
                class="fabric-btn-primary"
              >
                保存
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: all 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .relative,
.modal-leave-to .relative {
  transform: scale(0.95);
}
</style>
