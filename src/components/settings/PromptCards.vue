<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useSettingsStore } from '@/stores/settings';
import type { PromptCard, PromptCardPosition } from '@/stores/settings';
import PromptCardEditModal from './PromptCardEditModal.vue';
import PromptPreviewModal from './PromptPreviewModal.vue';

const settingsStore = useSettingsStore();

const showEditModal = ref(false);
const showPreviewModal = ref(false);
const editingCard = ref<PromptCard | null>(null);
const isNewCard = ref(false);

const activeTab = ref<PromptCardPosition>('prefix');
const newCardDefaultPosition = ref<PromptCardPosition | undefined>(undefined);

const isLoading = computed(() => settingsStore.isLoadingPromptCards);
const prefixCards = computed(() => settingsStore.prefixCards);
const suffixCards = computed(() => settingsStore.suffixCards);

const currentCards = computed(() =>
  activeTab.value === 'prefix' ? prefixCards.value : suffixCards.value
);

function openNewCard() {
  editingCard.value = null;
  isNewCard.value = true;
  newCardDefaultPosition.value = activeTab.value;
  showEditModal.value = true;
}

function openEditCard(card: PromptCard) {
  editingCard.value = card;
  isNewCard.value = false;
  newCardDefaultPosition.value = undefined;
  showEditModal.value = true;
}

async function handleSaveCard(card: PromptCard) {
  await settingsStore.upsertPromptCard(card);
  showEditModal.value = false;
}

async function handleDeleteCard(id: string) {
  await settingsStore.deletePromptCard(id);
}

async function handleToggle(id: string) {
  await settingsStore.togglePromptCard(id);
}

async function handleMove(id: string, direction: 'up' | 'down') {
  await settingsStore.movePromptCard(id, direction);
}

onMounted(async () => {
  await settingsStore.loadPromptCards();
});
</script>

<template>
  <div class="space-y-4">
    <!-- Header with actions -->
    <div class="flex items-center justify-end gap-2">
      <button
        @click="showPreviewModal = true"
        class="p-2 text-fabric-thread/50 hover:text-primary-600 hover:bg-primary-100 rounded-lg transition-all duration-200 disabled:opacity-30 disabled:cursor-not-allowed"
        :disabled="prefixCards.length === 0 && suffixCards.length === 0"
        title="预览最终提示词"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
        </svg>
      </button>
      <button @click="openNewCard" class="fabric-btn-primary inline-flex flex-row items-center gap-1.5">
        <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        <span>新建卡片</span>
      </button>
    </div>

    <!-- Loading -->
    <div v-if="isLoading" class="flex justify-center py-8">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
    </div>

    <template v-else>
      <!-- Tab Bar -->
      <div class="flex gap-2">
        <button
          @click="activeTab = 'prefix'"
          :class="[
            'flex-1 px-3 py-2.5 text-sm font-medium rounded-lg transition-all duration-220',
            activeTab === 'prefix'
              ? 'bg-primary-500/15 text-primary-700 border border-primary-500/30 shadow-sm'
              : 'text-fabric-thread hover:bg-fabric-canvas/50'
          ]"
        >
          <span class="flex items-center justify-center gap-1.5">
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 15l7-7 7 7" />
            </svg>
            前置卡片
            <span class="ml-0.5 text-xs opacity-75">({{ prefixCards.length }})</span>
          </span>
        </button>
        <button
          @click="activeTab = 'suffix'"
          :class="[
            'flex-1 px-3 py-2.5 text-sm font-medium rounded-lg transition-all duration-220',
            activeTab === 'suffix'
              ? 'bg-amber-500/15 text-amber-700 border border-amber-500/30 shadow-sm'
              : 'text-fabric-thread hover:bg-fabric-canvas/50'
          ]"
        >
          <span class="flex items-center justify-center gap-1.5">
            <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M19 9l-7 7-7-7" />
            </svg>
            后置卡片
            <span class="ml-0.5 text-xs opacity-75">({{ suffixCards.length }})</span>
          </span>
        </button>
      </div>

      <!-- Tab Content with Transition -->
      <Transition name="tab-content" mode="out-in">
        <div :key="activeTab">
          <!-- Empty State -->
          <div
            v-if="currentCards.length === 0"
            class="text-center py-12 text-fabric-thread/50 bg-fabric-linen/20 rounded-xl border border-dashed border-fabric-sand/40"
          >
            <svg class="w-12 h-12 mx-auto mb-3 text-fabric-sand" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
            <p class="text-sm font-medium text-fabric-thread/60">
              {{ activeTab === 'prefix' ? '暂无前置卡片' : '暂无后置卡片' }}
            </p>
            <p class="text-xs mt-1">
              {{ activeTab === 'prefix' ? '前置卡片会添加在 Agent 系统提示词之前' : '后置卡片会添加在 Agent 系统提示词之后' }}
            </p>
          </div>

          <!-- Cards List -->
          <div
            v-else
            class="bg-fabric-warm rounded-xl border border-fabric-sand/40 overflow-hidden"
          >
            <div
              v-for="(card, index) in currentCards"
              :key="card.id"
              :class="[
                'flex items-center gap-3 px-4 py-3 hover:bg-fabric-linen/30 transition-colors duration-180',
                'border-l-[3px]',
                activeTab === 'prefix' ? 'border-l-primary-400' : 'border-l-amber-400',
                index > 0 ? 'border-t border-fabric-sand/20' : ''
              ]"
            >
              <!-- Toggle -->
              <button
                @click="handleToggle(card.id)"
                :class="[
                  'relative inline-flex h-5 w-9 items-center rounded-full transition-colors duration-200 shrink-0',
                  card.enabled ? 'bg-primary-500' : 'bg-fabric-sand/60'
                ]"
              >
                <span
                  :class="[
                    'inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform duration-200',
                    card.enabled ? 'translate-x-[18px]' : 'translate-x-[3px]'
                  ]"
                />
              </button>

              <!-- Title & Content Preview -->
              <div class="flex-1 min-w-0 cursor-pointer" @click="openEditCard(card)">
                <div class="flex items-center gap-2">
                  <span :class="['text-sm font-medium', card.enabled ? 'text-fabric-sepia' : 'text-fabric-thread/40']">
                    {{ card.title }}
                  </span>
                  <span class="text-xs text-fabric-thread/30">{{ card.content.length }} 字符</span>
                </div>
                <p class="text-xs text-fabric-thread/40 truncate mt-0.5">{{ card.content.substring(0, 80) }}{{ card.content.length > 80 ? '...' : '' }}</p>
              </div>

              <!-- Order Controls -->
              <div class="flex items-center gap-0.5 shrink-0">
                <button
                  @click="handleMove(card.id, 'up')"
                  :disabled="index === 0"
                  class="p-1 text-fabric-thread/40 hover:text-fabric-thread disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                  title="上移"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
                  </svg>
                </button>
                <button
                  @click="handleMove(card.id, 'down')"
                  :disabled="index === currentCards.length - 1"
                  class="p-1 text-fabric-thread/40 hover:text-fabric-thread disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                  title="下移"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                  </svg>
                </button>
              </div>

              <!-- Delete -->
              <button
                @click="handleDeleteCard(card.id)"
                class="p-1 text-fabric-thread/30 hover:text-red-500 transition-colors shrink-0"
                title="删除"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
            </div>
          </div>
        </div>
      </Transition>

      <!-- How it works - compact version -->
      <div class="flex items-center gap-3 px-3 py-2 bg-fabric-linen/20 rounded-lg text-xs text-fabric-thread/50">
        <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span class="font-mono">[前置卡片] + [Agent 内置提示词] + [后置卡片]</span>
      </div>
    </template>

    <!-- Edit Modal -->
    <PromptCardEditModal
      :visible="showEditModal"
      :card="editingCard"
      :is-new="isNewCard"
      :default-position="newCardDefaultPosition"
      @close="showEditModal = false"
      @save="handleSaveCard"
    />

    <!-- Preview Modal -->
    <PromptPreviewModal
      :visible="showPreviewModal"
      :prefix-cards="prefixCards"
      :suffix-cards="suffixCards"
      @close="showPreviewModal = false"
    />
  </div>
</template>

<style scoped>
.tab-content-enter-active {
  transition: opacity 150ms ease-out;
}

.tab-content-leave-active {
  transition: opacity 100ms ease-in;
}

.tab-content-enter-from,
.tab-content-leave-to {
  opacity: 0;
}
</style>
