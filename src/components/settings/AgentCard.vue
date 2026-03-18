<script setup lang="ts">
import type { AgentConfig, AgentKind } from '@/stores/settings';
import { BUILT_IN_AGENT_LABELS, type BuiltInAgentKind } from '@/constants/labels';

const props = defineProps<{
  agent: AgentConfig;
  providerName?: string;
}>();

const emit = defineEmits<{
  (e: 'edit'): void;
  (e: 'delete'): void;
  (e: 'toggle-enabled'): void;
}>();

function getAgentKindLabel(kind: AgentKind): string {
  if (kind === 'custom') return '自定义';
  if (typeof kind === 'object' && 'built_in' in kind) {
    return BUILT_IN_AGENT_LABELS[kind.built_in as BuiltInAgentKind] || kind.built_in;
  }
  return '未知';
}

function getAgentKindColor(kind: AgentKind): string {
  if (kind === 'custom') return 'bg-fabric-sand/50 text-fabric-thread';
  if (typeof kind === 'object' && 'built_in' in kind) {
    const colors: Record<string, string> = {
      technique_analysis: 'bg-accent-technique/15 text-accent-technique',
      character_extraction: 'bg-accent-character/15 text-accent-character',
      setting_extraction: 'bg-accent-setting/15 text-accent-setting',
      event_extraction: 'bg-accent-event/15 text-accent-event',
    };
    return colors[kind.built_in] || 'bg-fabric-sand/50 text-fabric-thread';
  }
  return 'bg-fabric-sand/50 text-fabric-thread';
}

function isBuiltIn(kind: AgentKind): boolean {
  return typeof kind === 'object' && 'built_in' in kind;
}
</script>

<template>
  <div
    :class="[
      'bg-fabric-warm rounded-xl border p-4 transition-all duration-220',
      agent.enabled
        ? 'border-fabric-sand/40 hover:shadow-fabric'
        : 'border-fabric-sand/30 opacity-60',
    ]"
  >
    <!-- Header -->
    <div class="flex items-start justify-between mb-3">
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2 mb-1 flex-wrap">
          <h3 class="font-semibold text-fabric-sepia truncate">{{ agent.name }}</h3>
          <span
            :class="['px-2 py-0.5 text-xs rounded-full whitespace-nowrap shrink-0', getAgentKindColor(agent.kind)]"
          >
            {{ getAgentKindLabel(agent.kind) }}
          </span>
        </div>
        <p class="text-sm text-fabric-thread/60">
          {{ providerName || agent.provider_id }} · {{ agent.model }}
        </p>
      </div>

      <!-- Toggle -->
      <button
        @click="emit('toggle-enabled')"
        role="switch"
        :aria-checked="agent.enabled"
        :aria-label="`${agent.name} Agent 开关`"
        :class="[
          'relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-220 ease-in-out focus:outline-none',
          agent.enabled ? 'bg-primary-500' : 'bg-fabric-sand',
        ]"
      >
        <span
          :class="[
            'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-220 ease-in-out',
            agent.enabled ? 'translate-x-5' : 'translate-x-0',
          ]"
        />
      </button>
    </div>

    <!-- Details -->
    <div class="space-y-2 text-sm">
      <div class="flex items-center gap-4 text-fabric-thread/70">
        <span class="flex items-center gap-1">
          <svg class="w-4 h-4 text-fabric-thread/50" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
          </svg>
          温度: {{ agent.temperature }}
        </span>
        <span v-if="agent.max_tokens" class="flex items-center gap-1">
          <svg class="w-4 h-4 text-fabric-thread/50" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16m-7 6h7" />
          </svg>
          最大 Token: {{ agent.max_tokens }}
        </span>
      </div>

      <!-- System Prompt Preview -->
      <div v-if="agent.system_prompt" class="mt-2">
        <p class="text-xs text-fabric-thread/50 mb-1">系统提示词:</p>
        <p class="text-xs text-fabric-thread/70 line-clamp-2 bg-fabric-linen/50 p-2 rounded">
          {{ agent.system_prompt }}
        </p>
      </div>
      <div v-else-if="isBuiltIn(agent.kind)" class="mt-2">
        <p class="text-xs text-fabric-thread/50 italic">使用内置提示词</p>
      </div>
    </div>

    <!-- Actions -->
    <div class="flex items-center justify-end gap-2 mt-4 pt-3 border-t border-fabric-sand/30">
      <button
        @click="emit('edit')"
        class="px-3 py-1.5 text-sm text-fabric-thread/70 hover:text-fabric-sepia hover:bg-fabric-sand/30 rounded-lg transition-colors duration-220"
      >
        编辑
      </button>
      <button
        v-if="!isBuiltIn(agent.kind)"
        @click="emit('delete')"
        class="px-3 py-1.5 text-sm text-red-600 hover:text-red-700 hover:bg-red-50 rounded-lg transition-colors duration-220"
      >
        删除
      </button>
    </div>
  </div>
</template>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
