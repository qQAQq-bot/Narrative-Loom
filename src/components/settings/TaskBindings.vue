<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useSettingsStore } from '@/stores/settings';
import type { TaskType, TaskBindings as TaskBindingsType, AgentConfig } from '@/stores/settings';
import FabricSelect from '@/components/ui/FabricSelect.vue';
import { BUILT_IN_AGENT_LABELS, type BuiltInAgentKind } from '@/constants/labels';

const settingsStore = useSettingsStore();

const DEFAULT_BINDINGS: Record<TaskType, string> = {
  technique_analysis: '',
  character_extraction: '',
  setting_extraction: '',
  event_extraction: '',
  style_analysis: '',
};

// Local bindings state
const localBindings = ref<Record<TaskType, string>>({
  ...DEFAULT_BINDINGS,
});

const hasChanges = ref(false);
const isSaving = ref(false);

const taskTypes: { type: TaskType; label: string; description: string }[] = [
  { type: 'technique_analysis', label: BUILT_IN_AGENT_LABELS.technique_analysis, description: '分析章节中使用的写作技法' },
  { type: 'character_extraction', label: BUILT_IN_AGENT_LABELS.character_extraction, description: '从章节中提取人物信息' },
  { type: 'setting_extraction', label: BUILT_IN_AGENT_LABELS.setting_extraction, description: '从章节中提取世界观设定' },
  { type: 'event_extraction', label: BUILT_IN_AGENT_LABELS.event_extraction, description: '从章节中提取关键事件' },
  { type: 'style_analysis', label: BUILT_IN_AGENT_LABELS.style_analysis, description: '分析章节的写作风格特征' },
];

// Computed
const agents = computed(() => settingsStore.agents);
const enabledAgents = computed(() => settingsStore.enabledAgents);
const isLoading = computed(() => settingsStore.isLoadingAgents);

// Get agent name by ID
function getAgentName(agentId: string): string {
  const agent = agents.value.find(a => a.id === agentId);
  return agent?.name || agentId;
}

// Get suitable agents for a task type
function getSuitableAgents(taskType: TaskType): AgentConfig[] {
  return enabledAgents.value.filter(agent => {
    // Custom agents can be used for any task
    if (agent.kind === 'custom') return true;
    // Built-in agents match their task type
    if (typeof agent.kind === 'object' && 'built_in' in agent.kind) {
      return agent.kind.built_in === taskType;
    }
    return false;
  });
}

// Get options for FabricSelect with groups
function getAgentOptions(taskType: TaskType) {
  const suitable = getSuitableAgents(taskType);
  const recommended = suitable.filter(a =>
    typeof a.kind === 'object' && 'built_in' in a.kind && a.kind.built_in === taskType
  );
  const others = suitable.filter(a =>
    a.kind === 'custom' ||
    (typeof a.kind === 'object' && 'built_in' in a.kind && a.kind.built_in !== taskType)
  );

  const options: { label: string; options: { value: string; label: string }[] }[] = [];

  // Add "未绑定" option as a group with single item
  options.push({
    label: '状态',
    options: [{ value: '', label: '未绑定' }]
  });

  if (recommended.length > 0) {
    options.push({
      label: '推荐 (匹配类型)',
      options: recommended.map(a => ({ value: a.id, label: a.name }))
    });
  }

  if (others.length > 0) {
    options.push({
      label: '其他可用',
      options: others.map(a => ({ value: a.id, label: a.name }))
    });
  }

  return options;
}

// Update local binding
function updateBinding(taskType: TaskType, agentId: string) {
  localBindings.value[taskType] = agentId;
  hasChanges.value = true;
}

// Save bindings
async function saveBindings() {
  isSaving.value = true;
  try {
    await settingsStore.saveTaskBindings({
      bindings: { ...localBindings.value },
    });
    hasChanges.value = false;
  } catch (e) {
    console.error('Failed to save bindings:', e);
  } finally {
    isSaving.value = false;
  }
}

// Reset to saved bindings
function resetBindings() {
  localBindings.value = { ...DEFAULT_BINDINGS, ...(settingsStore.taskBindings.bindings as Partial<Record<TaskType, string>>) };
  hasChanges.value = false;
}

// Initialize from store
watch(
  () => settingsStore.taskBindings,
  (newBindings) => {
    if (newBindings?.bindings) {
      localBindings.value = { ...DEFAULT_BINDINGS, ...(newBindings.bindings as Partial<Record<TaskType, string>>) };
    }
  },
  { immediate: true }
);

onMounted(async () => {
  if (agents.value.length === 0) {
    await settingsStore.loadAgents();
  }
  await settingsStore.loadTaskBindings();
});
</script>

<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div></div>
      <div v-if="hasChanges" class="flex items-center gap-2">
        <button
          @click="resetBindings"
          class="px-3 py-1.5 text-sm text-fabric-thread/70 hover:bg-fabric-sand/30 rounded-lg transition-colors duration-220"
        >
          取消
        </button>
        <button
          @click="saveBindings"
          :disabled="isSaving"
          class="fabric-btn-primary disabled:opacity-50"
        >
          {{ isSaving ? '保存中...' : '保存更改' }}
        </button>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="isLoading" class="flex justify-center py-8">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
    </div>

    <!-- Bindings Table -->
    <div v-else class="bg-fabric-warm rounded-xl border border-fabric-sand/40 overflow-hidden">
      <table class="w-full">
        <thead class="bg-fabric-linen/50 border-b border-fabric-sand/30">
          <tr>
            <th class="px-4 py-3 text-left text-sm font-medium text-fabric-sepia">任务类型</th>
            <th class="px-4 py-3 text-left text-sm font-medium text-fabric-sepia">描述</th>
            <th class="px-4 py-3 text-left text-sm font-medium text-fabric-sepia">绑定的 Agent</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-fabric-sand/20">
          <tr v-for="task in taskTypes" :key="task.type" class="hover:bg-fabric-linen/30 transition-colors duration-180">
            <td class="px-4 py-3">
              <span class="font-medium text-fabric-sepia">{{ task.label }}</span>
            </td>
            <td class="px-4 py-3">
              <span class="text-sm text-fabric-thread/60">{{ task.description }}</span>
            </td>
            <td class="px-4 py-3">
              <FabricSelect
                :model-value="localBindings[task.type]"
                @update:model-value="updateBinding(task.type, $event)"
                :options="getAgentOptions(task.type)"
                placeholder="未绑定"
                size="sm"
              />
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- No Agents Warning -->
    <div
      v-if="!isLoading && enabledAgents.length === 0"
      class="p-4 bg-amber-50 border border-amber-200 rounded-lg"
    >
      <div class="flex items-start gap-3">
        <svg class="w-5 h-5 text-amber-500 shrink-0 mt-0.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
        </svg>
        <div>
          <h4 class="text-sm font-medium text-amber-800">没有可用的 Agent</h4>
          <p class="text-sm text-amber-700 mt-1">
            请先在上方添加并启用 Agent，然后才能配置任务绑定。
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
