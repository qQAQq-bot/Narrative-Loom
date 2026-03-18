<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import type { AgentConfig, AgentKind, OutputMode, ProviderWithStatus } from '@/stores/settings';
import FabricSelect from '@/components/ui/FabricSelect.vue';
import { BUILT_IN_AGENT_LABELS } from '@/constants/labels';

const props = defineProps<{
  isOpen: boolean;
  agent: AgentConfig | null;
  providers: ProviderWithStatus[];
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'save', agent: AgentConfig): void;
}>();

// Form state
const formData = ref<AgentConfig>({
  id: '',
  name: '',
  kind: 'custom',
  enabled: true,
  provider_id: '',
  model: '',
  temperature: 0.7,
  max_tokens: null,
  system_prompt: null,
  output_mode: 'json_object',
});

const isEditing = computed(() => !!props.agent);

const builtInTypes = Object.entries(BUILT_IN_AGENT_LABELS).map(([value, label]) => ({
  value,
  label,
}));

const outputModes = [
  { value: 'text', label: '纯文本' },
  { value: 'json_object', label: 'JSON 对象' },
];

// Get available models for selected provider
const availableModels = computed(() => {
  const provider = props.providers.find(p => p.id === formData.value.provider_id);
  return provider?.available_models || [];
});

// Check if current kind is built-in
const isBuiltInKind = computed(() => {
  return typeof formData.value.kind === 'object' && 'built_in' in formData.value.kind;
});

const currentBuiltInType = computed(() => {
  if (isBuiltInKind.value && typeof formData.value.kind === 'object') {
    return (formData.value.kind as { built_in: string }).built_in;
  }
  return null;
});

// Form validation
const isValid = computed(() => {
  return (
    formData.value.name.trim() !== '' &&
    formData.value.provider_id !== '' &&
    formData.value.model !== ''
  );
});

// Initialize form when modal opens or agent changes
watch(
  () => [props.isOpen, props.agent],
  () => {
    if (props.isOpen) {
      if (props.agent) {
        formData.value = { ...props.agent };
      } else {
        // Reset to defaults for new agent
        formData.value = {
          id: `agent-${Date.now()}`,
          name: '',
          kind: 'custom',
          enabled: true,
          provider_id: props.providers[0]?.id || '',
          model: '',
          temperature: 0.7,
          max_tokens: null,
          system_prompt: null,
          output_mode: 'json_object',
        };
      }
    }
  },
  { immediate: true }
);

// Update model when provider changes
watch(
  () => formData.value.provider_id,
  (newProviderId) => {
    const provider = props.providers.find(p => p.id === newProviderId);
    if (provider && !provider.available_models.includes(formData.value.model)) {
      formData.value.model = provider.default_model || provider.available_models[0] || '';
    }
  }
);

function handleSubmit() {
  if (!isValid.value) return;
  emit('save', { ...formData.value });
}

function handleClose() {
  emit('close');
}

function setBuiltInKind(typeValue: string) {
  formData.value.kind = { built_in: typeValue as any };
  formData.value.name = builtInTypes.find(t => t.value === typeValue)?.label || typeValue;
}

function setCustomKind() {
  formData.value.kind = 'custom';
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="isOpen"
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
        @click.self="handleClose"
      >
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-fabric-sepia/30 dark:bg-black/50 backdrop-blur-sm"></div>

        <!-- Modal -->
        <div class="relative bg-fabric-cream dark:bg-fabric-canvas rounded-2xl shadow-fabric-lg w-full max-w-lg max-h-[90vh] flex flex-col overflow-hidden border border-fabric-sand/50 dark:border-fabric-sand/30">
          <!-- Header -->
          <div class="flex items-center justify-between px-6 py-4 border-b border-fabric-sand/40 bg-fabric-linen/50 shrink-0">
            <h2 class="text-lg font-semibold text-fabric-sepia font-serif">
              {{ isEditing ? '编辑 Agent' : '添加 Agent' }}
            </h2>
            <button
              @click="handleClose"
              class="p-1 text-fabric-thread/60 hover:text-fabric-sepia rounded-lg hover:bg-fabric-sand/30 transition-colors duration-180"
            >
              <svg class="w-5 h-5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
                <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
              </svg>
            </button>
          </div>

          <!-- Content -->
          <div class="flex-1 overflow-y-auto p-6 bg-fabric-warm">
            <form @submit.prevent="handleSubmit" class="space-y-4">
              <!-- Agent Type (only for new agents) -->
              <div v-if="!isEditing">
                <label class="block text-sm font-medium text-fabric-sepia mb-2">Agent 类型</label>
                <div class="grid grid-cols-2 gap-2">
                  <button
                    type="button"
                    @click="setCustomKind"
                    :class="[
                      'px-3 py-2 text-sm rounded-lg border transition-colors duration-180 text-left',
                      !isBuiltInKind
                        ? 'border-primary-500 bg-primary-500/10 text-primary-700 dark:text-primary-300'
                        : 'border-fabric-sand/50 hover:border-fabric-sand bg-fabric-linen/50 text-fabric-thread',
                    ]"
                  >
                    自定义
                  </button>
                  <button
                    v-for="type in builtInTypes"
                    :key="type.value"
                    type="button"
                    @click="setBuiltInKind(type.value)"
                    :class="[
                      'px-3 py-2 text-sm rounded-lg border transition-colors duration-180 text-left',
                      currentBuiltInType === type.value
                        ? 'border-primary-500 bg-primary-500/10 text-primary-700 dark:text-primary-300'
                        : 'border-fabric-sand/50 hover:border-fabric-sand bg-fabric-linen/50 text-fabric-thread',
                    ]"
                  >
                    {{ type.label }}
                  </button>
                </div>
              </div>

              <!-- Name -->
              <div>
                <label class="block text-sm font-medium text-fabric-sepia mb-1">名称</label>
                <input
                  v-model="formData.name"
                  type="text"
                  class="fabric-input"
                  placeholder="Agent 名称"
                  required
                />
              </div>

              <!-- Provider -->
              <div>
                <label class="block text-sm font-medium text-fabric-sepia mb-1">Provider</label>
                <FabricSelect
                  v-model="formData.provider_id"
                  :options="providers.map(p => ({ value: p.id, label: p.name }))"
                  placeholder="选择 Provider"
                  searchable
                  search-placeholder="搜索 Provider..."
                />
              </div>

              <!-- Model -->
              <div>
                <label class="block text-sm font-medium text-fabric-sepia mb-1">模型</label>
                <FabricSelect
                  v-if="availableModels.length > 0"
                  v-model="formData.model"
                  :options="availableModels.map(m => ({ value: m, label: m }))"
                  placeholder="选择模型"
                  searchable
                  search-placeholder="搜索模型..."
                />
                <input
                  v-else
                  v-model="formData.model"
                  type="text"
                  class="fabric-input"
                  placeholder="模型名称"
                  required
                />
              </div>

              <!-- Temperature -->
              <div>
                <label class="block text-sm font-medium text-fabric-sepia mb-1">
                  温度 (Temperature): {{ formData.temperature }}
                </label>
                <input
                  v-model.number="formData.temperature"
                  type="range"
                  min="0"
                  max="2"
                  step="0.1"
                  class="w-full accent-primary-500"
                />
                <div class="flex justify-between text-xs text-fabric-thread/50">
                  <span>精确 (0)</span>
                  <span>创意 (2)</span>
                </div>
              </div>

              <!-- Max Tokens -->
              <div>
                <label class="block text-sm font-medium text-fabric-sepia mb-1">最大 Token 数 (可选)</label>
                <input
                  v-model.number="formData.max_tokens"
                  type="number"
                  min="1"
                  class="fabric-input"
                  placeholder="留空使用默认值"
                />
              </div>

              <!-- Output Mode -->
              <div>
                <label class="block text-sm font-medium text-fabric-sepia mb-1">输出模式</label>
                <FabricSelect
                  v-model="formData.output_mode"
                  :options="outputModes"
                />
              </div>

              <!-- System Prompt -->
              <div>
                <label class="block text-sm font-medium text-fabric-sepia mb-1">
                  系统提示词
                  <span v-if="isBuiltInKind" class="text-fabric-thread/50 font-normal">(可选，覆盖内置提示词)</span>
                </label>
                <textarea
                  v-model="formData.system_prompt"
                  rows="4"
                  class="fabric-input font-mono text-sm resize-none"
                  :placeholder="isBuiltInKind ? '留空使用内置提示词' : '输入系统提示词...'"
                ></textarea>
              </div>

              <!-- Enabled -->
              <div class="flex items-center gap-2">
                <input
                  v-model="formData.enabled"
                  type="checkbox"
                  id="agent-enabled"
                  class="w-4 h-4 text-primary-600 border-fabric-sand rounded focus:ring-primary-500 bg-fabric-warm"
                />
                <label for="agent-enabled" class="text-sm text-fabric-sepia">启用此 Agent</label>
              </div>
            </form>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-fabric-sand/40 bg-fabric-linen/50 shrink-0">
            <button
              @click="handleClose"
              class="px-4 py-2 text-fabric-thread/70 hover:bg-fabric-sand/30 rounded-lg transition-colors duration-180"
            >
              取消
            </button>
            <button
              @click="handleSubmit"
              :disabled="!isValid"
              class="fabric-btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {{ isEditing ? '保存' : '添加' }}
            </button>
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
