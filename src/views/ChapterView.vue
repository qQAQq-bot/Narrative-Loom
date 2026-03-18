<script setup lang="ts">
import { watch, onMounted } from 'vue';
import { useBookStore } from '@/stores/book';
import { useAnalysisStore } from '@/stores/analysis';
import ChapterReader from '@/components/reader/ChapterReader.vue';
import TechniqueCard from '@/components/cards/TechniqueCard.vue';
import KnowledgeCard from '@/components/cards/KnowledgeCard.vue';

const bookStore = useBookStore();
const analysisStore = useAnalysisStore();

// Sync analysis state and reload enabled agents when component mounts
onMounted(() => {
  analysisStore.syncAnalysisState();
  // Reload enabled agent types in case settings changed
  analysisStore.loadEnabledAgentTypes();
});

// Watch for chapter changes to load cards
watch(
  () => bookStore.currentChapterId,
  async (chapterId, oldChapterId) => {
    if (chapterId && chapterId !== oldChapterId) {
      // Load chapter content if not loaded
      if (!bookStore.currentChapter && !bookStore.isLoadingChapter) {
        await bookStore.loadChapter(chapterId);
      }
      // Load cards for this chapter
      await analysisStore.loadCardsForChapter(chapterId);
    } else if (!chapterId) {
      // Reset cards when no chapter selected
      analysisStore.reset();
    }
  },
  { immediate: true }
);

// Also watch for chapter content being loaded (for initial load)
watch(
  () => bookStore.currentChapter,
  async (chapter) => {
    if (chapter && bookStore.currentChapterId) {
      // Load cards if they haven't been loaded yet
      if (analysisStore.techniqueCards.length === 0 && analysisStore.knowledgeCards.length === 0) {
        await analysisStore.loadCardsForChapter(bookStore.currentChapterId);
      }
    }
  }
);

function handleTabChange(tab: 'technique' | 'knowledge') {
  analysisStore.setActiveTab(tab);
}

async function handleCollect(cardId: string) {
  try {
    await analysisStore.collectTechnique(cardId);
  } catch (e) {
    console.error('收藏失败', e);
  }
}

async function handleUncollect(cardId: string) {
  try {
    await analysisStore.uncollectTechnique(cardId);
  } catch (e) {
    console.error('取消收藏失败', e);
  }
}

async function handleUpdateKnowledgeStatus(cardId: string, status: 'pending' | 'accepted' | 'rejected' | 'merged') {
  try {
    await analysisStore.updateKnowledgeStatus(cardId, status);
  } catch (e) {
    console.error('更新状态失败', e);
  }
}

async function handleDeleteTechniqueCard(cardId: string) {
  try {
    await analysisStore.deleteTechniqueCard(cardId);
  } catch (e) {
    console.error('删除技法卡片失败', e);
  }
}

async function handleDeleteKnowledgeCard(cardId: string) {
  try {
    await analysisStore.deleteKnowledgeCard(cardId);
  } catch (e) {
    console.error('删除知识卡片失败', e);
  }
}

async function handleClearTechniqueCards() {
  if (!bookStore.currentChapterId) return;
  try {
    await analysisStore.clearChapterTechniqueCards(bookStore.currentChapterId);
  } catch (e) {
    console.error('清空技法卡片失败', e);
  }
}

async function handleClearKnowledgeCards() {
  if (!bookStore.currentChapterId) return;
  try {
    await analysisStore.clearChapterKnowledgeCards(bookStore.currentChapterId);
  } catch (e) {
    console.error('清空知识卡片失败', e);
  }
}

function handleCardSelect(cardId: string) {
  analysisStore.selectCard(cardId);
}

async function handleAnalyze() {
  if (!bookStore.currentChapterId) return;

  try {
    if (analysisStore.analysisMode === 'auto') {
      // Auto mode: run enabled agents in sequence (uses store's enabledAgentTypes)
      await analysisStore.analyzeAllAgents(bookStore.currentChapterId);
    } else {
      // Manual mode: just initialize statuses, user will click individual agents
      analysisStore.initAgentStatuses();
      console.log('Manual mode initialized, waiting for user to run agents');
    }
    // Reload chapter to update analyzed status
    await bookStore.loadChapter(bookStore.currentChapterId);
  } catch (e) {
    console.error('分析失败', e);
  }
}

async function handleReanalyze() {
  if (!bookStore.currentChapterId) return;

  try {
    if (analysisStore.analysisMode === 'auto') {
      // Auto mode: run enabled agents in sequence (uses store's enabledAgentTypes)
      await analysisStore.analyzeAllAgents(bookStore.currentChapterId);
    } else {
      // Manual mode: just initialize statuses, user will click individual agents
      analysisStore.initAgentStatuses();
    }
    // Reload chapter to update analyzed status
    await bookStore.loadChapter(bookStore.currentChapterId);
  } catch (e) {
    console.error('重新分析失败', e);
  }
}

function handleCancelAnalysis() {
  analysisStore.cancelAnalysis();
}

async function handleRunSingleAgent(agentType: string) {
  if (!bookStore.currentChapterId) return;

  try {
    await analysisStore.analyzeSingleAgent(
      bookStore.currentChapterId,
      agentType as 'technique' | 'character' | 'setting' | 'event'
    );
    // Reload chapter to update analyzed status if all agents completed
    await bookStore.loadChapter(bookStore.currentChapterId);
  } catch (e) {
    console.error(`Agent ${agentType} 分析失败`, e);
  }
}
</script>

<template>
  <div class="flex h-full min-h-0">
    <!-- Main Content - Chapter Reader -->
    <div class="flex-1 flex flex-col min-w-0 bg-white dark:bg-gray-800">
      <ChapterReader />
    </div>

    <!-- Right Sidebar - Analysis Cards -->
    <aside class="w-96 border-l border-fabric-sand/30 bg-fabric-linen flex flex-col h-full overflow-hidden">
      <!-- Tab Header -->
      <div class="p-4 border-b border-fabric-sand/30 bg-fabric-warm shrink-0">
        <div class="flex gap-2">
          <button
            @click="handleTabChange('technique')"
            :class="[
              'flex-1 px-3 py-2.5 text-sm font-medium rounded-lg transition-all duration-220',
              analysisStore.activeTab === 'technique'
                ? 'bg-accent-technique/15 text-accent-technique border border-accent-technique/30 shadow-sm'
                : 'text-fabric-thread hover:bg-fabric-canvas/50'
            ]"
          >
            <span class="flex items-center justify-center gap-1.5">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
              </svg>
              技法
              <span
                v-if="analysisStore.techniqueCount > 0"
                class="ml-0.5 text-xs opacity-75"
              >
                ({{ analysisStore.techniqueCount }})
              </span>
            </span>
          </button>
          <button
            @click="handleTabChange('knowledge')"
            :class="[
              'flex-1 px-3 py-2.5 text-sm font-medium rounded-lg transition-all duration-220',
              analysisStore.activeTab === 'knowledge'
                ? 'bg-accent-character/15 text-accent-character border border-accent-character/30 shadow-sm'
                : 'text-fabric-thread hover:bg-fabric-canvas/50'
            ]"
          >
            <span class="flex items-center justify-center gap-1.5">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
              </svg>
              知识
              <span
                v-if="analysisStore.knowledgeCount > 0"
                class="ml-0.5 text-xs opacity-75"
              >
                ({{ analysisStore.knowledgeCount }})
              </span>
            </span>
          </button>
        </div>
      </div>

      <!-- Cards List -->
      <div class="flex-1 overflow-y-auto p-4">
        <!-- Loading State -->
        <div
          v-if="analysisStore.isLoadingCards"
          class="text-center text-gray-400 dark:text-gray-500 py-8"
        >
          <span class="animate-pulse">加载卡片中...</span>
        </div>

        <!-- No Chapter Selected -->
        <div
          v-else-if="!bookStore.currentChapterId"
          class="text-center text-gray-400 dark:text-gray-500 py-8"
        >
          <div class="text-3xl mb-2">📋</div>
          <div>请先选择章节</div>
        </div>

        <!-- Technique Cards -->
        <div
          v-else-if="analysisStore.activeTab === 'technique'"
          class="space-y-3"
        >
          <!-- Clear button -->
          <div v-if="analysisStore.techniqueCards.length > 0" class="flex justify-end">
            <button
              @click="handleClearTechniqueCards"
              class="flex items-center gap-1 px-2 py-1 text-xs text-red-500 hover:text-red-600 hover:bg-red-50 rounded transition-colors"
            >
              <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
              清空
            </button>
          </div>
          <div
            v-if="analysisStore.techniqueCards.length === 0"
            class="text-center text-gray-400 dark:text-gray-500 py-8"
          >
            <div class="text-3xl mb-2">📝</div>
            <div>暂无技法卡片</div>
            <div class="text-sm mt-1">分析后将显示技法</div>
          </div>

          <TechniqueCard
            v-for="card in analysisStore.techniqueCards"
            :key="card.id"
            :card="card"
            :selected="analysisStore.selectedCardId === card.id"
            @select="handleCardSelect(card.id)"
            @collect="handleCollect(card.id)"
            @uncollect="handleUncollect(card.id)"
            @delete="handleDeleteTechniqueCard(card.id)"
          />
        </div>

        <!-- Knowledge Cards -->
        <div
          v-else
          class="space-y-3"
        >
          <!-- Clear button -->
          <div v-if="analysisStore.knowledgeCards.length > 0" class="flex justify-end">
            <button
              @click="handleClearKnowledgeCards"
              class="flex items-center gap-1 px-2 py-1 text-xs text-red-500 hover:text-red-600 hover:bg-red-50 rounded transition-colors"
            >
              <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
              清空
            </button>
          </div>
          <div
            v-if="analysisStore.knowledgeCards.length === 0"
            class="text-center text-gray-400 dark:text-gray-500 py-8"
          >
            <div class="text-3xl mb-2">📚</div>
            <div>暂无知识卡片</div>
            <div class="text-sm mt-1">分析后将显示知识</div>
          </div>

          <KnowledgeCard
            v-for="card in analysisStore.knowledgeCards"
            :key="card.id"
            :card="card"
            :selected="analysisStore.selectedCardId === card.id"
            @select="handleCardSelect(card.id)"
            @update-status="(status) => handleUpdateKnowledgeStatus(card.id, status)"
            @delete="handleDeleteKnowledgeCard(card.id)"
          />
        </div>
      </div>

      <!-- Footer - Analysis Panel -->
      <div class="p-4 border-t border-fabric-sand/30 bg-fabric-warm shrink-0">
        <!-- Analysis Mode Selector (when not analyzing) -->
        <div
          v-if="!analysisStore.isAnalyzing && bookStore.currentChapter"
          class="mb-3"
        >
          <div class="flex items-center gap-2 text-xs text-fabric-thread/70 mb-2">
            <span>分析模式:</span>
            <button
              class="px-2 py-1 rounded transition-colors"
              :class="analysisStore.analysisMode === 'auto'
                ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400'
                : 'hover:bg-fabric-sand/50'"
              @click="analysisStore.setAnalysisMode('auto')"
            >
              自动
            </button>
            <button
              class="px-2 py-1 rounded transition-colors"
              :class="analysisStore.analysisMode === 'manual'
                ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400'
                : 'hover:bg-fabric-sand/50'"
              @click="analysisStore.setAnalysisMode('manual')"
            >
              手动
            </button>
          </div>
        </div>

        <!-- Agent Status List (visible when analyzing or in manual mode with statuses) -->
        <div
          v-if="analysisStore.agentStatuses.length > 0 && (analysisStore.isAnalyzing || analysisStore.analysisMode === 'manual')"
          class="space-y-2 mb-3"
        >
          <div
            v-for="agent in analysisStore.agentStatuses"
            :key="agent.type"
            class="flex items-center justify-between py-1.5 px-3 rounded-lg text-sm"
            :class="{
              'bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800/30': agent.status === 'running',
              'bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800/30': agent.status === 'completed',
              'bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800/30': agent.status === 'error',
              'bg-fabric-sand/30 border border-fabric-sand/50': agent.status === 'pending',
            }"
          >
            <div class="flex items-center gap-2">
              <!-- Status Icon -->
              <svg v-if="agent.status === 'running'" class="animate-spin h-4 w-4 text-blue-600 dark:text-blue-400" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <svg v-else-if="agent.status === 'completed'" class="h-4 w-4 text-green-600 dark:text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
              </svg>
              <svg v-else-if="agent.status === 'error'" class="h-4 w-4 text-red-600 dark:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
              <div v-else class="h-4 w-4 rounded-full border-2 border-fabric-thread/30"></div>

              <span :class="{
                'text-blue-700 dark:text-blue-400': agent.status === 'running',
                'text-green-700 dark:text-green-400': agent.status === 'completed',
                'text-red-700 dark:text-red-400': agent.status === 'error',
                'text-fabric-thread/70': agent.status === 'pending',
              }">{{ agent.name }}</span>

              <span v-if="agent.resultCount !== undefined" class="text-xs text-fabric-thread/50">
                ({{ agent.resultCount }} 项)
              </span>
            </div>

            <!-- Manual Run Button - show for agents that are not running, allow multiple agents to run -->
            <button
              v-if="analysisStore.analysisMode === 'manual' && agent.status !== 'running'"
              class="px-2 py-0.5 text-xs rounded bg-fabric-paper hover:bg-fabric-sand/50 border border-fabric-sand/50 transition-colors"
              @click="handleRunSingleAgent(agent.type)"
            >
              执行
            </button>
          </div>
        </div>

        <!-- Analyzing state - show Cancel button -->
        <div
          v-if="analysisStore.isAnalyzing"
          class="space-y-2"
        >
          <button
            class="fabric-btn w-full flex items-center justify-center gap-2 text-sm text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20"
            @click="handleCancelAnalysis"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
            <span>取消分析</span>
          </button>
        </div>

        <!-- Not analyzed yet - show Analyze button -->
        <div
          v-else-if="bookStore.currentChapter && !bookStore.currentChapter.analyzed"
          class="space-y-2"
        >
          <button
            class="fabric-btn-primary w-full flex items-center justify-center gap-2"
            @click="handleAnalyze"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
            </svg>
            <span>{{ analysisStore.analysisMode === 'auto' ? '开始分析' : '初始化分析' }}</span>
          </button>
        </div>

        <!-- Already analyzed - show status and re-analyze button -->
        <div
          v-else-if="bookStore.currentChapter?.analyzed"
          class="space-y-3"
        >
          <div class="text-center py-2 px-4 rounded-lg bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800/30">
            <div class="flex items-center justify-center gap-2 text-green-700 dark:text-green-400">
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <span class="font-medium">本章已分析完成</span>
            </div>
          </div>
          <button
            class="fabric-btn w-full flex items-center justify-center gap-2 text-sm"
            @click="handleReanalyze"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            <span>重新分析</span>
          </button>
        </div>

        <!-- No chapter selected -->
        <div
          v-else
          class="text-center text-sm text-fabric-thread/60 py-2"
        >
          <div class="flex items-center justify-center gap-2">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span>选择章节后可开始分析</span>
          </div>
        </div>
      </div>
    </aside>
  </div>
</template>
