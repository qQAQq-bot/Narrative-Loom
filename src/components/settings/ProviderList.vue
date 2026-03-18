<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useSettingsStore, type ProviderWithStatus, type SaveProviderRequest, type TestConnectionResult } from '@/stores/settings';
import ProviderCard from './ProviderCard.vue';
import ProviderEditModal from './ProviderEditModal.vue';
import ConfirmModal from '@/components/ui/ConfirmModal.vue';
import Pagination from '@/components/ui/Pagination.vue';

const settingsStore = useSettingsStore();

// Pagination
const PAGE_SIZE = 6;
const currentPage = ref(1);

// Modal state
const showEditModal = ref(false);
const editingProvider = ref<ProviderWithStatus | null>(null);
const isNewProvider = ref(false);

// Delete confirmation state
const showDeleteConfirm = ref(false);
const deletingProviderId = ref<string | null>(null);
const deletingProviderName = ref<string>('');

// Test results
const testResults = ref<Record<string, TestConnectionResult>>({});

// Computed
const providers = computed(() => settingsStore.providers);

// Paginated providers
const paginatedProviders = computed(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE;
  return providers.value.slice(start, start + PAGE_SIZE);
});

// Adjust page when providers change
watch(() => providers.value.length, (newLen) => {
  const maxPage = Math.ceil(newLen / PAGE_SIZE);
  if (currentPage.value > maxPage && maxPage > 0) {
    currentPage.value = maxPage;
  }
});

onMounted(() => {
  if (providers.value.length === 0) {
    settingsStore.loadProviders();
  }
});

function handleAddProvider() {
  editingProvider.value = null;
  isNewProvider.value = true;
  showEditModal.value = true;
}

function handleEditProvider(provider: ProviderWithStatus) {
  editingProvider.value = provider;
  isNewProvider.value = false;
  showEditModal.value = true;
}

async function handleSaveProvider(data: SaveProviderRequest) {
  try {
    await settingsStore.saveProvider(data);
    showEditModal.value = false;
    editingProvider.value = null;
  } catch (e) {
    console.error('保存失败', e);
  }
}

function handleDeleteProvider(id: string) {
  // Find provider name for display
  const provider = providers.value.find(p => p.id === id);
  deletingProviderName.value = provider?.name || id;
  deletingProviderId.value = id;
  showDeleteConfirm.value = true;
}

async function confirmDelete() {
  if (!deletingProviderId.value) return;

  try {
    await settingsStore.deleteProvider(deletingProviderId.value);
    showDeleteConfirm.value = false;
    showEditModal.value = false;
    editingProvider.value = null;
    deletingProviderId.value = null;
  } catch (e) {
    console.error('删除失败', e);
  }
}

function cancelDelete() {
  showDeleteConfirm.value = false;
  deletingProviderId.value = null;
}

async function handleTestConnection(provider: ProviderWithStatus) {
  try {
    const result = await settingsStore.testConnection(provider.id);
    testResults.value[provider.id] = result;
  } catch (e) {
    testResults.value[provider.id] = {
      success: false,
      message: '测试失败',
      latency_ms: null,
    };
  }
}

async function handleToggleEnabled(provider: ProviderWithStatus) {
  try {
    await settingsStore.saveProvider({
      ...provider,
      enabled: !provider.enabled,
    });
  } catch (e) {
    console.error('切换状态失败', e);
  }
}

function closeModal() {
  showEditModal.value = false;
  editingProvider.value = null;
}
</script>

<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div></div>
      <button
        @click="handleAddProvider"
        class="fabric-btn-primary flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        <span>添加 Provider</span>
      </button>
    </div>

    <!-- Loading -->
    <div v-if="settingsStore.isLoadingProviders" class="text-center py-8 text-fabric-thread/60">
      <div class="animate-spin inline-block w-6 h-6 border-2 border-primary-500 border-t-transparent rounded-full mb-2"></div>
      <p>加载中...</p>
    </div>

    <!-- Empty State -->
    <div
      v-else-if="providers.length === 0"
      class="text-center py-12 border-2 border-dashed border-fabric-sand/60 rounded-xl bg-fabric-linen/30"
    >
      <div class="text-fabric-thread/50 mb-4">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="h-12 w-12 mx-auto"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z"
          />
        </svg>
      </div>
      <p class="text-fabric-thread/70 mb-4">还没有配置任何 Provider</p>
      <button
        @click="handleAddProvider"
        class="px-4 py-2 bg-primary-500/10 text-primary-600 rounded-lg hover:bg-primary-500/20 transition-colors duration-220 text-sm font-medium"
      >
        添加第一个 Provider
      </button>
    </div>

    <!-- Provider Grid -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <ProviderCard
        v-for="provider in paginatedProviders"
        :key="provider.id"
        :provider="provider"
        :is-testing="settingsStore.isTesting && settingsStore.testingProviderId === provider.id"
        :test-result="testResults[provider.id]"
        @edit="handleEditProvider(provider)"
        @test="handleTestConnection(provider)"
        @toggle-enabled="handleToggleEnabled(provider)"
      />
    </div>

    <!-- Pagination -->
    <Pagination
      v-if="!settingsStore.isLoadingProviders && providers.length > PAGE_SIZE"
      v-model:current-page="currentPage"
      :total-items="providers.length"
      :page-size="PAGE_SIZE"
    />

    <!-- Error -->
    <div
      v-if="settingsStore.error"
      class="bg-red-50 text-red-700 px-4 py-3 rounded-lg text-sm border border-red-200"
    >
      {{ settingsStore.error }}
    </div>

    <!-- Edit Modal -->
    <ProviderEditModal
      :visible="showEditModal"
      :provider="editingProvider"
      :is-new="isNewProvider"
      @close="closeModal"
      @save="handleSaveProvider"
      @delete="handleDeleteProvider"
    />

    <!-- Delete Confirmation Modal -->
    <ConfirmModal
      :visible="showDeleteConfirm"
      title="删除 Provider"
      :message="`确定要删除 Provider「${deletingProviderName}」吗？此操作不可恢复。`"
      confirm-text="删除"
      cancel-text="取消"
      type="danger"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </div>
</template>
