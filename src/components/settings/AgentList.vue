<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useSettingsStore } from '@/stores/settings';
import type { AgentConfig } from '@/stores/settings';
import AgentCard from './AgentCard.vue';
import AgentEditModal from './AgentEditModal.vue';
import ConfirmModal from '@/components/ui/ConfirmModal.vue';
import Pagination from '@/components/ui/Pagination.vue';

const settingsStore = useSettingsStore();

// Pagination
const PAGE_SIZE = 6;
const currentPage = ref(1);

// Modal state
const showEditModal = ref(false);
const editingAgent = ref<AgentConfig | null>(null);

// Delete confirmation state
const showDeleteConfirm = ref(false);
const deletingAgent = ref<AgentConfig | null>(null);

// Computed
const agents = computed(() => settingsStore.agents);
const providers = computed(() => settingsStore.providers);
const isLoading = computed(() => settingsStore.isLoadingAgents);

// Paginated agents
const paginatedAgents = computed(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE;
  return agents.value.slice(start, start + PAGE_SIZE);
});

// Reset to page 1 when agents change significantly
watch(() => agents.value.length, (newLen) => {
  const maxPage = Math.ceil(newLen / PAGE_SIZE);
  if (currentPage.value > maxPage && maxPage > 0) {
    currentPage.value = maxPage;
  }
});

// Get provider name by ID
function getProviderName(providerId: string): string {
  const provider = providers.value.find(p => p.id === providerId);
  return provider?.name || providerId;
}

// Actions
function openAddModal() {
  editingAgent.value = null;
  showEditModal.value = true;
}

function openEditModal(agent: AgentConfig) {
  editingAgent.value = agent;
  showEditModal.value = true;
}

function closeModal() {
  showEditModal.value = false;
  editingAgent.value = null;
}

async function handleSave(agent: AgentConfig) {
  try {
    await settingsStore.saveAgent(agent);
    closeModal();
  } catch (e) {
    console.error('Failed to save agent:', e);
  }
}

function handleDelete(agent: AgentConfig) {
  deletingAgent.value = agent;
  showDeleteConfirm.value = true;
}

async function confirmDelete() {
  if (!deletingAgent.value) return;

  try {
    await settingsStore.deleteAgent(deletingAgent.value.id);
    showDeleteConfirm.value = false;
    deletingAgent.value = null;
  } catch (e) {
    console.error('Failed to delete agent:', e);
  }
}

function cancelDelete() {
  showDeleteConfirm.value = false;
  deletingAgent.value = null;
}

async function handleToggleEnabled(agent: AgentConfig) {
  try {
    await settingsStore.saveAgent({
      ...agent,
      enabled: !agent.enabled,
    });
  } catch (e) {
    console.error('Failed to toggle agent:', e);
  }
}

onMounted(async () => {
  if (agents.value.length === 0) {
    await settingsStore.loadAgents();
  }
  if (providers.value.length === 0) {
    await settingsStore.loadProviders();
  }
});
</script>

<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div></div>
      <button
        @click="openAddModal"
        class="fabric-btn-primary flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        添加 Agent
      </button>
    </div>

    <!-- Loading -->
    <div v-if="isLoading" class="flex justify-center py-8">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
    </div>

    <!-- Empty State -->
    <div
      v-else-if="agents.length === 0"
      class="text-center py-12 bg-fabric-linen/30 rounded-xl border-2 border-dashed border-fabric-sand/60"
    >
      <svg class="w-12 h-12 mx-auto text-fabric-thread/40 mb-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
      </svg>
      <h4 class="text-lg font-medium text-fabric-sepia mb-2">还没有配置 Agent</h4>
      <p class="text-sm text-fabric-thread/60 mb-4">添加 Agent 来执行章节分析任务</p>
      <button
        @click="openAddModal"
        class="px-4 py-2 bg-primary-500/10 text-primary-600 rounded-lg hover:bg-primary-500/20 transition-colors duration-220 text-sm font-medium"
      >
        添加第一个 Agent
      </button>
    </div>

    <!-- Agent Grid -->
    <div v-else class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <AgentCard
        v-for="agent in paginatedAgents"
        :key="agent.id"
        :agent="agent"
        :provider-name="getProviderName(agent.provider_id)"
        @edit="openEditModal(agent)"
        @delete="handleDelete(agent)"
        @toggle-enabled="handleToggleEnabled(agent)"
      />
    </div>

    <!-- Pagination -->
    <Pagination
      v-if="!isLoading && agents.length > PAGE_SIZE"
      v-model:current-page="currentPage"
      :total-items="agents.length"
      :page-size="PAGE_SIZE"
    />

    <!-- Edit Modal -->
    <AgentEditModal
      :is-open="showEditModal"
      :agent="editingAgent"
      :providers="providers"
      @close="closeModal"
      @save="handleSave"
    />

    <!-- Delete Confirmation Modal -->
    <ConfirmModal
      :visible="showDeleteConfirm"
      title="删除 Agent"
      :message="`确定要删除 Agent「${deletingAgent?.name || ''}」吗？此操作不可恢复。`"
      confirm-text="删除"
      cancel-text="取消"
      type="danger"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </div>
</template>
