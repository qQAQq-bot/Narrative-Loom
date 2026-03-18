<script setup lang="ts">
import { computed } from 'vue';
import type { ProviderWithStatus } from '@/stores/settings';

const props = defineProps<{
  provider: ProviderWithStatus;
  isTesting?: boolean;
  testResult?: { success: boolean; message: string; latency_ms: number | null } | null;
}>();

const emit = defineEmits<{
  (e: 'edit'): void;
  (e: 'test'): void;
  (e: 'toggle-enabled'): void;
}>();

const apiFormatLabel = computed(() => {
  const format = props.provider.api_format;
  switch (format) {
    case 'openai':
      return 'OpenAI';
    case 'anthropic':
      return 'Anthropic';
    case 'ollama':
      return 'Ollama';
    case 'chat_completions':
      return 'Chat Completions';
    case 'responses':
      return 'Responses';
    default:
      return format;
  }
});

const statusBadgeClass = computed(() => {
  if (!props.provider.enabled) {
    return 'bg-fabric-sand/50 text-fabric-thread/70';
  }
  if (!props.provider.has_api_key && props.provider.api_key_ref) {
    return 'bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400';
  }
  return 'bg-accent-character/20 text-accent-character';
});

const statusLabel = computed(() => {
  if (!props.provider.enabled) {
    return '已禁用';
  }
  if (!props.provider.has_api_key && props.provider.api_key_ref) {
    return '缺少 API Key';
  }
  return '就绪';
});
</script>

<template>
  <div
    :class="[
      'rounded-xl border p-4 transition-all duration-220',
      provider.enabled
        ? 'bg-fabric-warm border-fabric-sand/40 hover:border-primary-400/50 hover:shadow-fabric'
        : 'bg-fabric-linen/50 border-fabric-sand/30 opacity-75',
    ]"
  >
    <!-- Header -->
    <div class="flex items-start justify-between mb-3">
      <div class="flex items-center gap-3">
        <div
          :class="[
            'w-10 h-10 rounded-lg flex items-center justify-center text-lg font-bold font-serif',
            provider.enabled
              ? 'bg-primary-500/10 text-primary-600'
              : 'bg-fabric-sand/30 text-fabric-thread/50',
          ]"
        >
          {{ provider.name.charAt(0).toUpperCase() }}
        </div>
        <div>
          <h3 class="font-semibold text-fabric-sepia">{{ provider.name }}</h3>
          <p class="text-xs text-fabric-thread/60 font-mono">{{ provider.id }}</p>
        </div>
      </div>

      <span
        :class="['text-xs px-2 py-1 rounded-full font-medium', statusBadgeClass]"
      >
        {{ statusLabel }}
      </span>
    </div>

    <!-- Info -->
    <div class="space-y-2 text-sm mb-4">
      <div class="flex items-center justify-between">
        <span class="text-fabric-thread/60">API 地址</span>
        <span class="text-fabric-sepia font-mono text-xs truncate max-w-[200px]">
          {{ provider.base_url }}
        </span>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-fabric-thread/60">API 格式</span>
        <span class="text-fabric-sepia">{{ apiFormatLabel }}</span>
      </div>
      <div class="flex items-center justify-between">
        <span class="text-fabric-thread/60">默认模型</span>
        <span class="text-fabric-sepia font-mono text-xs">{{ provider.default_model }}</span>
      </div>
      <div v-if="provider.masked_api_key" class="flex items-center justify-between">
        <span class="text-fabric-thread/60">API Key</span>
        <span class="text-fabric-sepia font-mono text-xs flex items-center gap-1">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-accent-character" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z" clip-rule="evenodd" />
          </svg>
          {{ provider.masked_api_key }}
        </span>
      </div>
    </div>

    <!-- Test Result -->
    <div
      v-if="testResult"
      :class="[
        'mb-4 px-3 py-2 rounded-lg text-sm',
        testResult.success
          ? 'bg-accent-character/10 text-accent-character'
          : 'bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400',
      ]"
    >
      <div class="flex items-center gap-2">
        <span>{{ testResult.success ? '✓' : '✗' }}</span>
        <span>{{ testResult.message }}</span>
        <span v-if="testResult.latency_ms" class="text-xs opacity-75">
          ({{ testResult.latency_ms }}ms)
        </span>
      </div>
    </div>

    <!-- Actions -->
    <div class="flex items-center justify-between pt-3 border-t border-fabric-sand/30">
      <button
        @click="emit('toggle-enabled')"
        :class="[
          'text-sm px-3 py-1.5 rounded-lg transition-colors duration-220',
          provider.enabled
            ? 'text-fabric-thread/70 hover:bg-fabric-sand/30'
            : 'text-primary-600 hover:bg-primary-500/10',
        ]"
      >
        {{ provider.enabled ? '禁用' : '启用' }}
      </button>

      <div class="flex items-center gap-2">
        <button
          @click="emit('test')"
          :disabled="isTesting"
          class="text-sm px-3 py-1.5 rounded-lg text-fabric-thread/70 hover:bg-fabric-sand/30 transition-colors duration-220 disabled:opacity-50"
        >
          <span v-if="isTesting" class="flex items-center gap-1">
            <span class="animate-spin">⟳</span>
            测试中...
          </span>
          <span v-else>测试连接</span>
        </button>
        <button
          @click="emit('edit')"
          class="text-sm px-3 py-1.5 rounded-lg bg-primary-500/10 text-primary-600 hover:bg-primary-500/20 transition-colors duration-220"
        >
          编辑
        </button>
      </div>
    </div>
  </div>
</template>
