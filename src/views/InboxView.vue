<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import InboxCardModal from '@/components/inbox/InboxCardModal.vue'
import { formatKnowledgeType, formatConfidence } from '@/constants/labels'

interface InboxItem {
  id: string
  chapter_id: string
  knowledge_type: string
  title: string
  content: Record<string, unknown>
  evidence: string[]
  confidence: string
  created_at: string
}

interface InboxStats {
  total: number
  characters: number
  settings: number
  events: number
}

const route = useRoute()
const bookId = computed(() => route.params.id as string)

const items = ref<InboxItem[]>([])
const stats = ref<InboxStats>({ total: 0, characters: 0, settings: 0, events: 0 })
const loading = ref(true)
const error = ref<string | null>(null)
const activeFilter = ref<string | null>(null)
const processingIds = ref<Set<string>>(new Set())

// Modal state
const showModal = ref(false)
const modalMode = ref<'edit' | 'merge'>('edit')
const selectedItem = ref<InboxItem | null>(null)

const filteredItems = computed(() => {
  if (!activeFilter.value) return items.value
  return items.value.filter(item => item.knowledge_type === activeFilter.value)
})

const filters = [
  { key: null, label: '全部' },
  { key: 'character', label: '人物' },
  { key: 'setting', label: '设定' },
  { key: 'event', label: '事件' },
]

async function fetchData() {
  loading.value = true
  error.value = null
  try {
    const [inboxItems, inboxStats] = await Promise.all([
      invoke<InboxItem[]>('get_inbox', { bookId: bookId.value }),
      invoke<InboxStats>('get_inbox_stats', { bookId: bookId.value }),
    ])
    items.value = inboxItems
    stats.value = inboxStats
  } catch (e) {
    console.error('Failed to fetch inbox:', e)
    error.value = typeof e === 'string' ? e : '加载收件箱失败'
  } finally {
    loading.value = false
  }
}

async function acceptCard(cardId: string) {
  processingIds.value.add(cardId)
  try {
    await invoke('accept_card', { bookId: bookId.value, cardId })
    await fetchData()
  } catch (e) {
    console.error('Failed to accept card:', e)
    error.value = typeof e === 'string' ? e : '接受失败'
  } finally {
    processingIds.value.delete(cardId)
  }
}

async function rejectCard(cardId: string) {
  processingIds.value.add(cardId)
  try {
    await invoke('reject_card', { bookId: bookId.value, cardId })
    await fetchData()
  } catch (e) {
    console.error('Failed to reject card:', e)
    error.value = typeof e === 'string' ? e : '驳回失败'
  } finally {
    processingIds.value.delete(cardId)
  }
}

async function acceptAll() {
  const cardIds = filteredItems.value.map(item => item.id)
  if (cardIds.length === 0) return

  loading.value = true
  error.value = null
  try {
    await invoke('batch_accept_cards', { bookId: bookId.value, cardIds })
    await fetchData()
  } catch (e) {
    console.error('Failed to batch accept:', e)
    error.value = typeof e === 'string' ? e : '批量接受失败'
  } finally {
    loading.value = false
  }
}

function openEditModal(item: InboxItem) {
  selectedItem.value = item
  modalMode.value = 'edit'
  showModal.value = true
}

function openMergeModal(item: InboxItem) {
  selectedItem.value = item
  modalMode.value = 'merge'
  showModal.value = true
}

function closeModal() {
  showModal.value = false
  selectedItem.value = null
}

async function handleModalSaved() {
  await fetchData()
}

function clearError() {
  error.value = null
}

function getTypeBadgeClass(type: string) {
  switch (type) {
    case 'character': return 'bg-accent-character/10 text-accent-character'
    case 'setting': return 'bg-accent-setting/10 text-accent-setting'
    case 'event': return 'bg-accent-event/10 text-accent-event'
    default: return 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400'
  }
}

function formatDate(dateStr: string) {
  const date = new Date(dateStr)
  return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
}

onMounted(fetchData)
watch(bookId, fetchData)
</script>

<template>
  <div class="p-6 bg-gray-50 dark:bg-gray-900 h-full overflow-y-auto">
    <div class="flex items-center justify-between mb-6">
      <div>
        <h2 class="text-xl font-semibold text-gray-900 dark:text-white">收件箱</h2>
        <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
          共 {{ stats.total }} 条待审核
          <span v-if="stats.characters" class="ml-2">人物 {{ stats.characters }}</span>
          <span v-if="stats.settings" class="ml-2">设定 {{ stats.settings }}</span>
          <span v-if="stats.events" class="ml-2">事件 {{ stats.events }}</span>
        </p>
      </div>
      <button
        class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 transition-colors"
        :disabled="filteredItems.length === 0 || loading"
        @click="acceptAll"
      >
        全部接受 ({{ filteredItems.length }})
      </button>
    </div>

    <!-- Error Banner -->
    <div
      v-if="error"
      class="mb-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 px-4 py-3 rounded-xl flex items-center justify-between"
    >
      <div class="flex items-center gap-3">
        <svg class="w-5 h-5 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span class="text-sm">{{ error }}</span>
      </div>
      <button @click="clearError" class="text-red-500 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300">
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <div class="flex gap-2 mb-6">
      <button
        v-for="filter in filters"
        :key="filter.key ?? 'all'"
        class="px-4 py-1.5 text-sm rounded-full transition-colors"
        :class="activeFilter === filter.key
          ? 'bg-gray-800 dark:bg-gray-200 text-white dark:text-gray-900'
          : 'bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700'"
        @click="activeFilter = filter.key"
      >
        {{ filter.label }}
        <span v-if="filter.key === null" class="ml-1 opacity-70">({{ stats.total }})</span>
        <span v-else-if="filter.key === 'character'" class="ml-1 opacity-70">({{ stats.characters }})</span>
        <span v-else-if="filter.key === 'setting'" class="ml-1 opacity-70">({{ stats.settings }})</span>
        <span v-else-if="filter.key === 'event'" class="ml-1 opacity-70">({{ stats.events }})</span>
      </button>
    </div>

    <div v-if="loading" class="flex justify-center py-12">
      <div class="animate-spin rounded-full h-8 w-8 border-2 border-gray-300 dark:border-gray-600 border-t-gray-600 dark:border-t-gray-300"></div>
    </div>

    <div v-else-if="filteredItems.length === 0" class="text-center py-12 bg-white dark:bg-gray-800 rounded-2xl border-2 border-dashed border-gray-200 dark:border-gray-700">
      <div class="text-gray-400 dark:text-gray-500 text-4xl mb-4">
        <svg class="w-16 h-16 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
        </svg>
      </div>
      <p class="text-gray-500 dark:text-gray-400">暂无待审核的条目</p>
      <p class="text-gray-400 dark:text-gray-500 text-sm mt-1">分析章节后，发现的知识会出现在这里</p>
    </div>

    <div v-else class="space-y-4">
      <div
        v-for="item in filteredItems"
        :key="item.id"
        class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl p-4 hover:shadow-md transition-shadow"
        :class="{ 'opacity-50': processingIds.has(item.id) }"
      >
        <div class="flex items-start justify-between">
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2 mb-2">
              <span
                class="px-2 py-0.5 text-xs font-medium rounded-full"
                :class="getTypeBadgeClass(item.knowledge_type)"
              >
                {{ formatKnowledgeType(item.knowledge_type) }}
              </span>
              <span class="text-xs text-gray-400 dark:text-gray-500">
                置信度: {{ formatConfidence(item.confidence) }}
              </span>
              <span class="text-xs text-gray-400 dark:text-gray-500">
                {{ formatDate(item.created_at) }}
              </span>
            </div>

            <h3 class="font-medium text-gray-900 dark:text-white mb-1">{{ item.title }}</h3>

            <p v-if="item.content.description" class="text-sm text-gray-600 dark:text-gray-400 line-clamp-2 mb-2">
              {{ item.content.description }}
            </p>

            <div v-if="item.evidence.length > 0" class="flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500">
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              {{ item.evidence.length }} 条证据
            </div>
          </div>

          <div class="flex items-center gap-2 ml-4">
            <!-- Edit Button -->
            <button
              class="px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors disabled:opacity-50"
              :disabled="processingIds.has(item.id)"
              @click="openEditModal(item)"
              title="编辑后接受"
            >
              <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
              </svg>
            </button>
            <!-- Merge Button -->
            <button
              class="px-3 py-1.5 text-sm text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition-colors disabled:opacity-50"
              :disabled="processingIds.has(item.id)"
              @click="openMergeModal(item)"
              title="合并到已有条目"
            >
              <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
              </svg>
            </button>
            <!-- Reject Button -->
            <button
              class="px-3 py-1.5 text-sm text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors disabled:opacity-50"
              :disabled="processingIds.has(item.id)"
              @click="rejectCard(item.id)"
            >
              驳回
            </button>
            <!-- Accept Button -->
            <button
              class="px-3 py-1.5 text-sm bg-green-600 text-white hover:bg-green-700 rounded-lg transition-colors disabled:opacity-50"
              :disabled="processingIds.has(item.id)"
              @click="acceptCard(item.id)"
            >
              接受
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Edit/Merge Modal -->
    <InboxCardModal
      :visible="showModal"
      :book-id="bookId"
      :item="selectedItem"
      :mode="modalMode"
      @close="closeModal"
      @saved="handleModalSaved"
    />
  </div>
</template>
