<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import type { PromptCard } from '@/stores/settings';
import { BUILT_IN_AGENT_LABELS } from '@/constants/labels';
import FabricSelect from '@/components/ui/FabricSelect.vue';

const props = defineProps<{
  visible: boolean;
  prefixCards: PromptCard[];
  suffixCards: PromptCard[];
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

type AgentType = 'technique' | 'character' | 'setting' | 'event';

const selectedAgent = ref<AgentType>('character');

const agentOptions = [
  { value: 'technique', label: BUILT_IN_AGENT_LABELS.technique_analysis },
  { value: 'character', label: BUILT_IN_AGENT_LABELS.character_extraction },
  { value: 'setting', label: BUILT_IN_AGENT_LABELS.setting_extraction },
  { value: 'event', label: BUILT_IN_AGENT_LABELS.event_extraction },
];

const enabledPrefixCards = computed(() =>
  props.prefixCards.filter(c => c.enabled).sort((a, b) => a.order - b.order)
);

const enabledSuffixCards = computed(() =>
  props.suffixCards.filter(c => c.enabled).sort((a, b) => a.order - b.order)
);

const prefixText = computed(() =>
  enabledPrefixCards.value.map(c => c.content.trim()).join('\n\n')
);

const suffixText = computed(() =>
  enabledSuffixCards.value.map(c => c.content.trim()).join('\n\n')
);

const hasContent = computed(() =>
  enabledPrefixCards.value.length > 0 || enabledSuffixCards.value.length > 0
);

watch(() => props.visible, (val) => {
  if (val) {
    selectedAgent.value = 'character';
  }
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
      @click.self="emit('close')"
    >
      <div class="bg-fabric-warm rounded-xl border border-fabric-sand/40 shadow-xl w-full max-w-2xl mx-4 max-h-[80vh] flex flex-col">
        <!-- Header -->
        <div class="flex items-center justify-between px-6 py-4 border-b border-fabric-sand/30 shrink-0">
          <h3 class="text-lg font-medium text-fabric-sepia">预览最终提示词</h3>
          <button
            @click="emit('close')"
            class="text-fabric-thread/50 hover:text-fabric-thread transition-colors"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Body -->
        <div class="px-6 py-4 overflow-y-auto flex-1">
          <!-- Agent Selector -->
          <div class="mb-4">
            <label class="block text-sm font-medium text-fabric-sepia mb-2">Agent 类型</label>
            <div class="w-48">
              <FabricSelect
                v-model="selectedAgent"
                :options="agentOptions"
                size="sm"
              />
            </div>
          </div>

          <!-- Preview Content -->
          <div class="bg-fabric-linen/30 rounded-lg border border-fabric-sand/30 p-4 font-mono text-sm space-y-4">
            <!-- No cards enabled -->
            <div v-if="!hasContent" class="text-fabric-thread/50 text-center py-4">
              没有启用的提示词卡片，Agent 将只使用内置系统提示词
            </div>

            <template v-else>
              <!-- Prefix Section -->
              <div v-if="prefixText">
                <div class="flex items-center gap-2 mb-2">
                  <span class="text-xs font-medium text-primary-600 bg-primary-100 px-2 py-0.5 rounded">前置卡片</span>
                  <span class="text-xs text-fabric-thread/40">{{ enabledPrefixCards.length }} 张</span>
                </div>
                <div class="text-fabric-sepia whitespace-pre-wrap break-words">{{ prefixText }}</div>
              </div>

              <!-- Separator -->
              <div class="border-t border-dashed border-fabric-sand/50 my-4"></div>

              <!-- Agent Prompt Placeholder -->
              <div>
                <div class="flex items-center gap-2 mb-2">
                  <span class="text-xs font-medium text-fabric-thread/60 bg-fabric-sand/40 px-2 py-0.5 rounded">Agent 内置提示词</span>
                </div>
                <div class="text-fabric-thread/50 italic">
                  [{{ agentOptions.find(o => o.value === selectedAgent)?.label }} 的内置系统提示词将在运行时插入此处]
                </div>
              </div>

              <!-- Separator -->
              <div v-if="suffixText" class="border-t border-dashed border-fabric-sand/50 my-4"></div>

              <!-- Suffix Section -->
              <div v-if="suffixText">
                <div class="flex items-center gap-2 mb-2">
                  <span class="text-xs font-medium text-amber-600 bg-amber-100 px-2 py-0.5 rounded">后置卡片</span>
                  <span class="text-xs text-fabric-thread/40">{{ enabledSuffixCards.length }} 张</span>
                </div>
                <div class="text-fabric-sepia whitespace-pre-wrap break-words">{{ suffixText }}</div>
              </div>
            </template>
          </div>

          <!-- Stats -->
          <div class="mt-3 flex items-center justify-between text-xs text-fabric-thread/40">
            <span>
              启用卡片: {{ enabledPrefixCards.length + enabledSuffixCards.length }} 张
            </span>
            <span>
              总字符数: {{ prefixText.length + suffixText.length }}
            </span>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex items-center justify-end px-6 py-4 border-t border-fabric-sand/30 shrink-0">
          <button
            @click="emit('close')"
            class="fabric-btn"
          >
            关闭
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
