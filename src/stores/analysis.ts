import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { useTauri } from '@/composables/useTauri';
import { useBookStore } from './book';
import {
  AGENT_TYPES as AGENT_TYPES_CONST,
  AGENT_TYPE_LABELS,
  type AgentType,
} from '@/constants/labels';

export interface TechniqueCardInfo {
  id: string;
  chapter_id: string;
  technique_type: string;
  title: string;
  description: string;
  mechanism: string;
  evidence: string[];
  tags: string[];
  collected: boolean;
  created_at: string;
}

// Valid knowledge card status values (must match backend)
export type KnowledgeCardStatus = 'pending' | 'accepted' | 'rejected' | 'merged';

export interface KnowledgeCardInfo {
  id: string;
  chapter_id: string;
  knowledge_type: string;
  title: string;
  content: Record<string, unknown>;
  evidence: string[];
  confidence: string;
  status: KnowledgeCardStatus;
  created_at: string;
}

export type CardTab = 'technique' | 'knowledge';

export interface ActiveEvidence {
  cardId: string;
  evidenceIndex: number;
  excerpt: string;
}

export type AgentStatusType = 'pending' | 'running' | 'completed' | 'error';
export type AnalysisMode = 'auto' | 'manual';

export interface AgentStatus {
  type: AgentType;
  name: string;
  status: AgentStatusType;
  resultCount?: number;
  error?: string;
}

export { type AgentType } from '@/constants/labels';

export const AGENT_TYPES = AGENT_TYPES_CONST.map(a => ({ type: a.type, name: a.label }));

const AGENT_TYPE_NAMES: Record<string, string> = {
  ...AGENT_TYPE_LABELS,
  starting: '准备中',
};

// Backend batch analysis progress event structure
interface BatchAnalysisProgressEvent {
  book_id: string;
  current_chapter_id: string;
  current_chapter_index: number;
  total_chapters: number;
  current_agent_type: string;
  status: string; // "running", "completed", "error", "cancelled"
  error: string | null;
}

// Store the current analysis promise outside of the reactive state
// to prevent it from being cancelled when navigating away
let currentAnalysisPromise: Promise<void> | null = null;
let currentAnalysisChapterId: string | null = null;
let currentAnalysisBookId: string | null = null;
// Track the currently running agent type at module level
// This survives across navigation since it's outside reactive state
let currentRunningAgentType: AgentType | null = null;
// Track if we're in auto mode (called from analyzeAllAgents)
let isInAutoMode = false;

export const useAnalysisStore = defineStore('analysis', () => {
  const { invoke, listen } = useTauri();
  const bookStore = useBookStore();

  // Current cards state
  const techniqueCards = ref<TechniqueCardInfo[]>([]);
  const knowledgeCards = ref<KnowledgeCardInfo[]>([]);
  const hasStyleObservation = ref(false); // Track if style observation exists for current chapter

  // UI state
  const activeTab = ref<CardTab>('technique');
  const selectedCardId = ref<string | null>(null);
  const isAnalyzing = ref(false);
  const analyzingChapterId = ref<string | null>(null);

  // Enabled agent types from settings (controls which agents are available for analysis)
  const enabledAgentTypes = ref<AgentType[]>([]);

  // Step-by-step analysis state
  const analysisMode = ref<AnalysisMode>('auto');
  const agentStatuses = ref<AgentStatus[]>([]);
  const currentAgentType = ref<AgentType | null>(null);

  // Evidence highlight state
  const activeEvidence = ref<ActiveEvidence | null>(null);
  const showEvidenceModal = ref(false);

  // Loading states
  const isLoadingCards = ref(false);
  const error = ref<string | null>(null);
  const analysisCancelled = ref(false);

  // Batch analysis state
  const batchMode = ref(false);
  const selectedChapterIds = ref<Set<string>>(new Set());
  const isBatchAnalyzing = ref(false);
  const batchProgress = ref({
    currentIndex: 0,
    totalCount: 0,
    currentChapterTitle: '',
    currentAgentType: '',
    isRunning: false,
    completedChapters: [] as string[],
    failedChapters: [] as { id: string; title: string; error: string }[],
  });

  // Load enabled agent types from settings
  async function loadEnabledAgentTypes() {
    try {
      const agents = await invoke<string[]>('get_enabled_agents');
      enabledAgentTypes.value = agents as AgentType[];
    } catch (e) {
      console.error('Failed to load enabled agents:', e);
      // Fallback to all agents
      enabledAgentTypes.value = ['technique', 'character', 'setting', 'event', 'style'];
    }
  }

  // Initialize agent statuses
  // If agentTypesToInit is provided, only initialize those agents
  // Otherwise, use enabledAgentTypes (for showing only enabled agents in UI)
  function initAgentStatuses(agentTypesToInit?: AgentType[]) {
    // Use provided types, or enabled types, or all types as fallback
    const typesToUse = agentTypesToInit
      ?? (enabledAgentTypes.value.length > 0 ? enabledAgentTypes.value : undefined);

    const agentsToShow = typesToUse
      ? AGENT_TYPES.filter(a => typesToUse.includes(a.type))
      : AGENT_TYPES;

    agentStatuses.value = agentsToShow.map(agent => ({
      type: agent.type,
      name: agent.name,
      status: 'pending' as AgentStatusType,
      resultCount: undefined,
      error: undefined,
    }));
  }

  // Update agent statuses based on loaded cards
  function updateAgentStatusesFromCards() {
    // Check technique cards
    const techniqueAgent = agentStatuses.value.find(a => a.type === 'technique');
    if (techniqueAgent && techniqueCards.value.length > 0) {
      techniqueAgent.status = 'completed';
      techniqueAgent.resultCount = techniqueCards.value.length;
    }

    // Check character cards
    const characterAgent = agentStatuses.value.find(a => a.type === 'character');
    const characterCards = knowledgeCards.value.filter(c => c.knowledge_type === 'character');
    if (characterAgent && characterCards.length > 0) {
      characterAgent.status = 'completed';
      characterAgent.resultCount = characterCards.length;
    }

    // Check setting cards
    const settingAgent = agentStatuses.value.find(a => a.type === 'setting');
    const settingCards = knowledgeCards.value.filter(c => c.knowledge_type === 'setting');
    if (settingAgent && settingCards.length > 0) {
      settingAgent.status = 'completed';
      settingAgent.resultCount = settingCards.length;
    }

    // Check event cards
    const eventAgent = agentStatuses.value.find(a => a.type === 'event');
    const eventCards = knowledgeCards.value.filter(c => c.knowledge_type === 'event');
    if (eventAgent && eventCards.length > 0) {
      eventAgent.status = 'completed';
      eventAgent.resultCount = eventCards.length;
    }

    // Check style observation
    const styleAgent = agentStatuses.value.find(a => a.type === 'style');
    if (styleAgent && hasStyleObservation.value) {
      styleAgent.status = 'completed';
      styleAgent.resultCount = 1; // Style analysis produces one observation per chapter
    }
  }

  // Sync state from module-level variables on store creation
  // This restores state when navigating back to the page
  function syncAnalysisState() {
    // Check if analysis is in progress (auto mode with promise, or manual mode with chapter id and running agent)
    const hasAutoModeAnalysis = currentAnalysisPromise && currentAnalysisChapterId;
    const hasManualModeAnalysis = !isInAutoMode && currentAnalysisChapterId && currentRunningAgentType;

    if (hasAutoModeAnalysis || hasManualModeAnalysis) {
      console.log('Restoring analysis state for chapter:', currentAnalysisChapterId, 'agent:', currentRunningAgentType, 'autoMode:', isInAutoMode);
      isAnalyzing.value = true;
      analyzingChapterId.value = currentAnalysisChapterId;
      currentAgentType.value = currentRunningAgentType;

      // Initialize agent statuses if empty
      if (agentStatuses.value.length === 0) {
        initAgentStatuses();
      }

      // Restore running agent status if there's a current agent type
      if (currentRunningAgentType) {
        const agent = agentStatuses.value.find(a => a.type === currentRunningAgentType);
        if (agent && agent.status !== 'completed') {
          agent.status = 'running';
          console.log('Restored running status for agent:', currentRunningAgentType);
        }
      }
    }
  }

  // Helper to update the module-level running agent type
  function setCurrentRunningAgentType(agentType: AgentType | null) {
    currentRunningAgentType = agentType;
    currentAgentType.value = agentType;
  }

  // Call sync on store initialization
  syncAnalysisState();
  // Load enabled agent types from settings
  loadEnabledAgentTypes();

  // Computed
  const techniqueCount = computed(() => techniqueCards.value.length);
  const knowledgeCount = computed(() => knowledgeCards.value.length);
  const collectedTechniqueCount = computed(() => 
    techniqueCards.value.filter(c => c.collected).length
  );

  const selectedCard = computed(() => {
    if (!selectedCardId.value) return null;
    if (activeTab.value === 'technique') {
      return techniqueCards.value.find(c => c.id === selectedCardId.value) || null;
    } else {
      return knowledgeCards.value.find(c => c.id === selectedCardId.value) || null;
    }
  });

  const cardsByType = computed(() => {
    const grouped: Record<string, TechniqueCardInfo[]> = {};
    for (const card of techniqueCards.value) {
      if (!grouped[card.technique_type]) {
        grouped[card.technique_type] = [];
      }
      grouped[card.technique_type].push(card);
    }
    return grouped;
  });

  const knowledgeByType = computed(() => {
    const grouped: Record<string, KnowledgeCardInfo[]> = {};
    for (const card of knowledgeCards.value) {
      if (!grouped[card.knowledge_type]) {
        grouped[card.knowledge_type] = [];
      }
      grouped[card.knowledge_type].push(card);
    }
    return grouped;
  });

  // Actions
  async function loadCardsForChapter(chapterId: string) {
    if (!bookStore.currentBookId) {
      error.value = '请先选择书籍';
      return;
    }

    // Check if analysis is currently running for this chapter BEFORE any async operation
    // Support both auto mode (with promise) and manual mode (with chapter id and running agent)
    const hasAutoModeAnalysis = currentAnalysisPromise && currentAnalysisChapterId === chapterId;
    const hasManualModeAnalysis = !isInAutoMode && currentAnalysisChapterId === chapterId && currentRunningAgentType;
    const isCurrentlyAnalyzing = hasAutoModeAnalysis || hasManualModeAnalysis;
    const runningAgentType = isCurrentlyAnalyzing ? currentRunningAgentType : null;

    console.log('loadCardsForChapter called:', chapterId, {
      currentAnalysisPromise: !!currentAnalysisPromise,
      currentAnalysisChapterId,
      currentRunningAgentType,
      isAnalyzing: isAnalyzing.value,
      isCurrentlyAnalyzing,
      isInAutoMode,
    });

    // If analysis is running, ensure isAnalyzing stays true
    if (isCurrentlyAnalyzing) {
      isAnalyzing.value = true;
      analyzingChapterId.value = chapterId;
    }

    isLoadingCards.value = true;
    error.value = null;

    try {
      const [techniques, knowledge, styleObs] = await Promise.all([
        invoke<TechniqueCardInfo[]>('get_technique_cards', {
          bookId: bookStore.currentBookId,
          chapterId,
        }),
        invoke<KnowledgeCardInfo[]>('get_knowledge_cards', {
          bookId: bookStore.currentBookId,
          chapterId,
        }),
        invoke<unknown | null>('get_style_observation', {
          bookId: bookStore.currentBookId,
          chapterId,
        }),
      ]);

      techniqueCards.value = techniques;
      knowledgeCards.value = knowledge;
      hasStyleObservation.value = styleObs !== null;

      // Reset agent statuses when loading a new chapter, then update based on loaded cards
      initAgentStatuses();
      updateAgentStatusesFromCards();

      // If analysis is running for this chapter, restore the running state
      if (isCurrentlyAnalyzing && runningAgentType) {
        const agent = agentStatuses.value.find(a => a.type === runningAgentType);
        if (agent && agent.status !== 'completed') {
          agent.status = 'running';
          console.log('loadCardsForChapter restored running status for:', runningAgentType);
        }
        // Ensure isAnalyzing stays true
        isAnalyzing.value = true;
        analyzingChapterId.value = chapterId;
      }
    } catch (e) {
      error.value = '加载卡片失败';
      console.error(e);
    } finally {
      isLoadingCards.value = false;
    }
  }

  async function collectTechnique(cardId: string) {
    if (!bookStore.currentBookId) return;

    try {
      await invoke<boolean>('collect_technique', {
        bookId: bookStore.currentBookId,
        cardId,
      });

      // Update local state
      const card = techniqueCards.value.find(c => c.id === cardId);
      if (card) {
        card.collected = true;
      }
    } catch (e) {
      console.error('收藏失败', e);
      throw e;
    }
  }

  async function uncollectTechnique(cardId: string) {
    if (!bookStore.currentBookId) return;

    try {
      await invoke<boolean>('uncollect_technique', {
        bookId: bookStore.currentBookId,
        cardId,
      });

      // Update local state
      const card = techniqueCards.value.find(c => c.id === cardId);
      if (card) {
        card.collected = false;
      }
    } catch (e) {
      console.error('取消收藏失败', e);
      throw e;
    }
  }

  async function updateKnowledgeStatus(cardId: string, status: KnowledgeCardStatus) {
    if (!bookStore.currentBookId) return;

    try {
      await invoke<boolean>('update_knowledge_card_status', {
        bookId: bookStore.currentBookId,
        cardId,
        status,
      });

      // Update local state
      const card = knowledgeCards.value.find(c => c.id === cardId);
      if (card) {
        card.status = status;
      }
    } catch (e) {
      console.error('更新状态失败', e);
      throw e;
    }
  }

  function setActiveTab(tab: CardTab) {
    activeTab.value = tab;
    selectedCardId.value = null;
  }

  function selectCard(cardId: string | null) {
    selectedCardId.value = cardId;
  }

  function reset() {
    techniqueCards.value = [];
    knowledgeCards.value = [];
    activeTab.value = 'technique';
    selectedCardId.value = null;
    isAnalyzing.value = false;
    error.value = null;
    activeEvidence.value = null;
    showEvidenceModal.value = false;
  }

  function highlightEvidence(cardId: string, evidenceIndex: number, excerpt: string) {
    activeEvidence.value = { cardId, evidenceIndex, excerpt };
  }

  function clearHighlight() {
    activeEvidence.value = null;
  }

  function openEvidenceModal(cardId: string, evidenceIndex: number, excerpt: string) {
    activeEvidence.value = { cardId, evidenceIndex, excerpt };
    showEvidenceModal.value = true;
  }

  function closeEvidenceModal() {
    showEvidenceModal.value = false;
  }

  interface AnalysisResult {
    chapter_id: string;
    technique_cards: TechniqueCardInfo[];
    knowledge_cards: KnowledgeCardInfo[];
    success: boolean;
    error: string | null;
  }

  async function analyzeChapter(chapterId: string, providerId?: string) {
    if (!bookStore.currentBookId) {
      error.value = '请先选择书籍';
      return;
    }

    // If already analyzing this chapter, don't start again
    if (isAnalyzing.value && analyzingChapterId.value === chapterId) {
      console.log('Already analyzing this chapter, skipping...');
      return;
    }

    isAnalyzing.value = true;
    analyzingChapterId.value = chapterId;
    error.value = null;

    const bookId = bookStore.currentBookId;

    // Create the analysis promise and store it outside reactive state
    // This ensures it continues even when navigating away
    const analysisPromise = (async () => {
      try {
        console.log('Starting analysis for chapter:', chapterId);
        const result = await invoke<AnalysisResult>('analyze_chapter', {
          bookId,
          chapterId,
          providerId,
        });

        console.log('Analysis completed:', result);

        if (result.success) {
          // Only update cards if we're still viewing this chapter
          if (analyzingChapterId.value === chapterId) {
            techniqueCards.value = result.technique_cards;
            knowledgeCards.value = result.knowledge_cards;
          }
        } else {
          if (analyzingChapterId.value === chapterId) {
            error.value = result.error || '分析失败';
          }
        }
      } catch (e) {
        console.error('Analysis error:', e);
        if (analyzingChapterId.value === chapterId) {
          error.value = '分析失败: ' + String(e);
        }
      } finally {
        // Only reset analyzing state if this is still the current analysis
        if (analyzingChapterId.value === chapterId) {
          isAnalyzing.value = false;
          analyzingChapterId.value = null;
        }
        currentAnalysisPromise = null;
        currentAnalysisChapterId = null;
        currentAnalysisBookId = null;
      }
    })();

    currentAnalysisPromise = analysisPromise;
    currentAnalysisChapterId = chapterId;
    currentAnalysisBookId = bookId;

    // Don't await here - let it run in background
    // The promise is stored in currentAnalysisPromise
    await analysisPromise;
  }

  async function cancelAnalysis() {
    if (isAnalyzing.value) {
      console.log('Cancelling analysis...');

      // Set the cancelled flag to stop the loop
      analysisCancelled.value = true;

      try {
        // Call backend to cancel the analysis (creates a flag file)
        const result = await invoke<{ cancelled: boolean; message: string }>('cancel_analysis');
        console.log('Cancel result:', result);

        if (result.cancelled) {
          error.value = '分析已取消，等待当前请求完成...';
        }
      } catch (e) {
        console.error('Failed to cancel analysis:', e);
        error.value = '取消分析失败';
      }

      // Reset the UI state
      isAnalyzing.value = false;
      analyzingChapterId.value = null;
      setCurrentRunningAgentType(null);
      currentAnalysisPromise = null;
      currentAnalysisChapterId = null;
      currentAnalysisBookId = null;
    }
  }

  // Single agent analysis result interface
  interface SingleAgentResult {
    agent_type: string;
    success: boolean;
    error: string | null;
    data: unknown[];
  }

  // Run a single agent analysis
  async function analyzeSingleAgent(chapterId: string, agentType: AgentType): Promise<boolean> {
    if (!bookStore.currentBookId) {
      error.value = '请先选择书籍';
      return false;
    }

    // Find the agent status
    const agentStatus = agentStatuses.value.find(a => a.type === agentType);
    if (!agentStatus) {
      console.error('Agent type not found:', agentType);
      return false;
    }

    // Check if this is a manual mode call (not called from analyzeAllAgents)
    const isManualMode = !isInAutoMode;

    // Set status to running
    agentStatus.status = 'running';
    agentStatus.error = undefined;
    setCurrentRunningAgentType(agentType);
    isAnalyzing.value = true;
    analyzingChapterId.value = chapterId;
    error.value = null;

    // For manual mode, set up module-level variables for navigation persistence
    if (isManualMode && !currentAnalysisChapterId) {
      currentAnalysisChapterId = chapterId;
      currentAnalysisBookId = bookStore.currentBookId;
    }

    try {
      console.log(`Running single agent: ${agentType}`);
      const result = await invoke<SingleAgentResult>('analyze_single_agent', {
        bookId: bookStore.currentBookId,
        chapterId,
        agentType,
      });

      console.log('Single agent result:', result);

      if (result.success) {
        agentStatus.status = 'completed';
        agentStatus.resultCount = result.data.length;

        // Reload cards to reflect new results
        await loadCardsForChapter(chapterId);

        return true;
      } else {
        agentStatus.status = 'error';
        agentStatus.error = result.error || '分析失败';
        error.value = result.error || '分析失败';
        return false;
      }
    } catch (e) {
      console.error('Single agent error:', e);
      agentStatus.status = 'error';
      agentStatus.error = String(e);
      error.value = String(e);
      return false;
    } finally {
      setCurrentRunningAgentType(null);

      // Check if any agents are still running
      const stillRunning = agentStatuses.value.some(a => a.status === 'running');

      // For manual mode, handle cleanup and chapter marking
      if (isManualMode) {
        // Check if all agents have completed (not pending or running)
        const allAgentsCompleted = agentStatuses.value.every(
          a => a.status === 'completed' || a.status === 'error'
        );

        // If all agents completed, mark chapter as analyzed
        if (allAgentsCompleted && bookStore.currentBookId) {
          try {
            await invoke('mark_chapter_analyzed', {
              bookId: bookStore.currentBookId,
              chapterId,
              techniqueCount: techniqueCards.value.length,
              knowledgeCount: knowledgeCards.value.length,
            });
            // Refresh chapter list and book info to update analyzed status
            await bookStore.loadChapters();
            await bookStore.refreshBook();
          } catch (e) {
            console.error('Failed to mark chapter as analyzed:', e);
          }
        }

        // Only clean up module-level variables if no agents are still running
        if (!stillRunning) {
          currentAnalysisChapterId = null;
          currentAnalysisBookId = null;
          isAnalyzing.value = false;
          analyzingChapterId.value = null;
        }
      } else {
        // For calls from analyzeAllAgents, only reset isAnalyzing if no other agents are running
        // and the overall analysis flow is not still in progress
        if (!stillRunning && !currentAnalysisPromise) {
          isAnalyzing.value = false;
          analyzingChapterId.value = null;
        }
      }
    }
  }

  // Run all agents in sequence (auto mode)
  async function analyzeAllAgents(chapterId: string, agentTypesToRun?: AgentType[]) {
    if (!bookStore.currentBookId) {
      error.value = '请先选择书籍';
      return;
    }

    // Determine which agents to run:
    // 1. Use provided types if specified
    // 2. Otherwise use enabled types from settings
    // 3. Fallback to all types if enabled types not loaded
    const typesToRun = agentTypesToRun
      ?? (enabledAgentTypes.value.length > 0 ? enabledAgentTypes.value : AGENT_TYPES.map(a => a.type));

    const agentsToRun = AGENT_TYPES.filter(a => typesToRun.includes(a.type));

    // Initialize only the agents that will actually run
    initAgentStatuses(agentsToRun.map(a => a.type));
    isAnalyzing.value = true;
    analyzingChapterId.value = chapterId;
    analysisCancelled.value = false;
    error.value = null;

    // Store in module-level variables to persist across navigation
    currentAnalysisChapterId = chapterId;
    currentAnalysisBookId = bookStore.currentBookId;
    isInAutoMode = true;

    // Create the analysis promise and store it
    const analysisPromise = (async () => {
      try {
        for (const agent of agentsToRun) {
          // Check if cancelled
          if (analysisCancelled.value) {
            console.log('Analysis cancelled, stopping');
            break;
          }

          // Ensure isAnalyzing stays true during the loop
          isAnalyzing.value = true;
          analyzingChapterId.value = chapterId;

          const success = await analyzeSingleAgent(chapterId, agent.type);
          if (!success) {
            console.log(`Agent ${agent.type} failed, continuing to next`);
          }
        }

        // Mark chapter as analyzed after all agents complete
        if (!analysisCancelled.value && bookStore.currentBookId) {
          try {
            await invoke('mark_chapter_analyzed', {
              bookId: bookStore.currentBookId,
              chapterId,
              techniqueCount: techniqueCards.value.length,
              knowledgeCount: knowledgeCards.value.length,
            });
            // Refresh chapter list and book info to update analyzed status
            await bookStore.loadChapters();
            await bookStore.refreshBook();
          } catch (e) {
            console.error('Failed to mark chapter as analyzed:', e);
          }
        }
      } finally {
        isAnalyzing.value = false;
        analyzingChapterId.value = null;
        analysisCancelled.value = false;

        // Clear module-level variables when done
        currentAnalysisPromise = null;
        currentAnalysisChapterId = null;
        currentAnalysisBookId = null;
        isInAutoMode = false;
        setCurrentRunningAgentType(null);
      }
    })();

    currentAnalysisPromise = analysisPromise;
    await analysisPromise;
  }

  // Set analysis mode
  function setAnalysisMode(mode: AnalysisMode) {
    analysisMode.value = mode;
  }

  // Check if analysis is in progress for a specific chapter
  function isAnalyzingChapter(chapterId: string): boolean {
    return isAnalyzing.value && analyzingChapterId.value === chapterId;
  }

  // Toggle batch mode
  function toggleBatchMode() {
    batchMode.value = !batchMode.value;
    if (!batchMode.value) {
      selectedChapterIds.value = new Set();
    }
  }

  // Toggle chapter selection for batch analysis
  function toggleChapterSelection(chapterId: string) {
    const newSet = new Set(selectedChapterIds.value);
    if (newSet.has(chapterId)) {
      newSet.delete(chapterId);
    } else {
      newSet.add(chapterId);
    }
    selectedChapterIds.value = newSet;
  }

  // Select all chapters in a volume
  function selectVolumeChapters(volumeTitle: string, chapters: { id: string; parent_title?: string }[]) {
    const newSet = new Set(selectedChapterIds.value);
    const volumeChapters = chapters.filter(c => c.parent_title === volumeTitle);
    for (const chapter of volumeChapters) {
      newSet.add(chapter.id);
    }
    selectedChapterIds.value = newSet;
  }

  // Clear all selections
  function clearChapterSelection() {
    selectedChapterIds.value = new Set();
  }

  // Reset batch progress
  function resetBatchProgress() {
    batchProgress.value = {
      currentIndex: 0,
      totalCount: 0,
      currentChapterTitle: '',
      currentAgentType: '',
      isRunning: false,
      completedChapters: [],
      failedChapters: [],
    };
  }

  // Start batch analysis
  async function startBatchAnalysis(chapterInfos: { id: string; title: string }[], agentTypes?: AgentType[]) {
    if (!bookStore.currentBookId) {
      error.value = '请先选择书籍';
      return;
    }

    if (chapterInfos.length === 0) {
      error.value = '请选择要分析的章节';
      return;
    }

    // Use provided types or enabled types from settings
    const typesToRun = agentTypes
      ?? (enabledAgentTypes.value.length > 0 ? enabledAgentTypes.value : AGENT_TYPES.map(a => a.type));

    // Create a map for quick chapter title lookup
    const chapterTitleMap = new Map(chapterInfos.map(c => [c.id, c.title]));

    isBatchAnalyzing.value = true;
    batchProgress.value = {
      currentIndex: 0,
      totalCount: chapterInfos.length,
      currentChapterTitle: chapterInfos[0]?.title || '',
      currentAgentType: '准备中',
      isRunning: true,
      completedChapters: [],
      failedChapters: [],
    };

    // Set up event listener for real-time progress updates
    let unlisten: (() => void) | null = null;
    try {
      unlisten = await listen<BatchAnalysisProgressEvent>('batch-analysis-progress', (payload) => {
        // Only process events for current book
        if (payload.book_id !== bookStore.currentBookId) return;

        // Update progress based on event
        batchProgress.value.currentIndex = payload.current_chapter_index;
        batchProgress.value.currentChapterTitle = chapterTitleMap.get(payload.current_chapter_id) || `章节 ${payload.current_chapter_index + 1}`;
        batchProgress.value.currentAgentType = AGENT_TYPE_NAMES[payload.current_agent_type] || payload.current_agent_type;

        // Handle different statuses
        if (payload.status === 'completed') {
          batchProgress.value.isRunning = false;
        } else if (payload.status === 'error' && payload.error) {
          console.warn('Batch analysis error:', payload.error);
        }
      });

      const result = await invoke<{
        completed_chapters: string[];
        failed_chapters: { id: string; agent_type: string; error: string }[];
        total: number;
      }>('batch_analyze_chapters', {
        bookId: bookStore.currentBookId,
        chapterIds: chapterInfos.map(c => c.id),
        agentTypes: typesToRun,
      });

      batchProgress.value.completedChapters = result.completed_chapters;
      batchProgress.value.failedChapters = result.failed_chapters.map(f => ({
        id: f.id,
        title: chapterInfos.find(c => c.id === f.id)?.title || f.id,
        error: f.error,
      }));
      batchProgress.value.currentIndex = result.total;
      batchProgress.value.isRunning = false;

      // Refresh chapter list and book info to update analyzed status
      await bookStore.loadChapters();
      if (bookStore.currentBookId) {
        await bookStore.refreshBook();
      }
    } catch (e) {
      console.error('Batch analysis error:', e);
      error.value = String(e);
      batchProgress.value.isRunning = false;
    } finally {
      // Clean up event listener
      if (unlisten) {
        unlisten();
      }
      isBatchAnalyzing.value = false;

      // 批量分析完成后退出多选模式
      batchMode.value = false;
      selectedChapterIds.value = new Set();
    }
  }

  // Cancel batch analysis (currently just resets state - actual cancellation would need backend support)
  function cancelBatchAnalysis() {
    isBatchAnalyzing.value = false;
    batchProgress.value.isRunning = false;
  }

  // Delete a single technique card
  async function deleteTechniqueCard(cardId: string) {
    if (!bookStore.currentBookId) return false;

    try {
      await invoke<boolean>('delete_technique_card', {
        bookId: bookStore.currentBookId,
        cardId,
      });

      // Update local state
      techniqueCards.value = techniqueCards.value.filter(c => c.id !== cardId);
      if (selectedCardId.value === cardId) {
        selectedCardId.value = null;
      }
      return true;
    } catch (e) {
      console.error('删除技法卡片失败', e);
      throw e;
    }
  }

  // Delete a single knowledge card
  async function deleteKnowledgeCard(cardId: string) {
    if (!bookStore.currentBookId) return false;

    try {
      await invoke<boolean>('delete_knowledge_card', {
        bookId: bookStore.currentBookId,
        cardId,
      });

      // Update local state
      knowledgeCards.value = knowledgeCards.value.filter(c => c.id !== cardId);
      if (selectedCardId.value === cardId) {
        selectedCardId.value = null;
      }
      return true;
    } catch (e) {
      console.error('删除知识卡片失败', e);
      throw e;
    }
  }

  // Clear all technique cards for a chapter
  async function clearChapterTechniqueCards(chapterId: string) {
    if (!bookStore.currentBookId) return 0;

    try {
      const count = await invoke<number>('clear_chapter_technique_cards', {
        bookId: bookStore.currentBookId,
        chapterId,
      });

      // Update local state
      techniqueCards.value = [];
      selectedCardId.value = null;

      // Reset technique agent status
      const techniqueAgent = agentStatuses.value.find(a => a.type === 'technique');
      if (techniqueAgent) {
        techniqueAgent.status = 'pending';
        techniqueAgent.resultCount = undefined;
      }

      return count;
    } catch (e) {
      console.error('清空技法卡片失败', e);
      throw e;
    }
  }

  // Clear all knowledge cards for a chapter
  async function clearChapterKnowledgeCards(chapterId: string) {
    if (!bookStore.currentBookId) return 0;

    try {
      const count = await invoke<number>('clear_chapter_knowledge_cards', {
        bookId: bookStore.currentBookId,
        chapterId,
      });

      // Update local state
      knowledgeCards.value = [];
      selectedCardId.value = null;

      // Reset knowledge agent statuses
      for (const agent of agentStatuses.value) {
        if (agent.type !== 'technique') {
          agent.status = 'pending';
          agent.resultCount = undefined;
        }
      }

      return count;
    } catch (e) {
      console.error('清空知识卡片失败', e);
      throw e;
    }
  }

  // Clear all cards for a chapter
  async function clearChapterAllCards(chapterId: string) {
    if (!bookStore.currentBookId) return { technique_cards_deleted: 0, knowledge_cards_deleted: 0 };

    try {
      const result = await invoke<{ technique_cards_deleted: number; knowledge_cards_deleted: number }>('clear_chapter_all_cards', {
        bookId: bookStore.currentBookId,
        chapterId,
      });

      // Update local state
      techniqueCards.value = [];
      knowledgeCards.value = [];
      selectedCardId.value = null;

      // Reset all agent statuses
      initAgentStatuses();

      return result;
    } catch (e) {
      console.error('清空所有卡片失败', e);
      throw e;
    }
  }

  async function clearAllTechniqueCards() {
    if (!bookStore.currentBookId) return 0;

    try {
      const result = await invoke<number>('clear_all_technique_cards', {
        bookId: bookStore.currentBookId,
      });

      // Clear local state if on technique tab
      techniqueCards.value = [];
      selectedCardId.value = null;

      return result;
    } catch (e) {
      console.error('清空全部技法卡片失败', e);
      throw e;
    }
  }

  async function clearAllKnowledgeCards() {
    if (!bookStore.currentBookId) return 0;

    try {
      const result = await invoke<number>('clear_all_knowledge_cards', {
        bookId: bookStore.currentBookId,
      });

      // Clear local state if on knowledge tab
      knowledgeCards.value = [];
      selectedCardId.value = null;

      return result;
    } catch (e) {
      console.error('清空全部知识卡片失败', e);
      throw e;
    }
  }

  return {
    // State
    techniqueCards,
    knowledgeCards,
    activeTab,
    selectedCardId,
    isAnalyzing,
    analyzingChapterId,
    isLoadingCards,
    error,
    activeEvidence,
    showEvidenceModal,
    enabledAgentTypes,

    // Step-by-step analysis state
    analysisMode,
    agentStatuses,
    currentAgentType,

    // Batch analysis state
    batchMode,
    selectedChapterIds,
    isBatchAnalyzing,
    batchProgress,

    // Computed
    techniqueCount,
    knowledgeCount,
    collectedTechniqueCount,
    selectedCard,
    cardsByType,
    knowledgeByType,

    // Actions
    loadCardsForChapter,
    analyzeChapter,
    cancelAnalysis,
    isAnalyzingChapter,
    syncAnalysisState,
    loadEnabledAgentTypes,
    collectTechnique,
    uncollectTechnique,
    updateKnowledgeStatus,
    setActiveTab,
    selectCard,
    reset,
    highlightEvidence,
    clearHighlight,
    openEvidenceModal,
    closeEvidenceModal,

    // Step-by-step analysis actions
    initAgentStatuses,
    updateAgentStatusesFromCards,
    analyzeSingleAgent,
    analyzeAllAgents,
    setAnalysisMode,

    // Batch analysis actions
    toggleBatchMode,
    toggleChapterSelection,
    selectVolumeChapters,
    clearChapterSelection,
    resetBatchProgress,
    startBatchAnalysis,
    cancelBatchAnalysis,

    // Delete actions
    deleteTechniqueCard,
    deleteKnowledgeCard,
    clearChapterTechniqueCards,
    clearChapterKnowledgeCards,
    clearChapterAllCards,
    clearAllTechniqueCards,
    clearAllKnowledgeCards,
  };
});
