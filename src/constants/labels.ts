/**
 * Chinese label mappings for English enum values from backend
 */

// Character role labels
export const ROLE_LABELS: Record<string, string> = {
  protagonist: '主角',
  antagonist: '反派',
  major: '主要角色',
  supporting: '配角',
  minor: '龙套',
};

// Setting type labels
export const SETTING_TYPE_LABELS: Record<string, string> = {
  location: '地点',
  organization: '组织',
  item: '物品',
  concept: '概念',
  power_system: '力量体系',
  era: '时代',
  worldview: '世界观',
};

// Event importance labels
export const IMPORTANCE_LABELS: Record<string, string> = {
  critical: '关键',
  major: '重要',
  normal: '普通',
  minor: '次要',
};

// Shared badge classes for event importance (used by Blueprint/Timeline, etc.)
export const IMPORTANCE_CLASSES: Record<string, string> = {
  critical: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  major: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  normal: 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400',
  minor: 'bg-gray-50 text-gray-500 dark:bg-gray-800 dark:text-gray-500',
};

// Knowledge type labels
export const KNOWLEDGE_TYPE_LABELS: Record<string, string> = {
  character: '人物',
  setting: '设定',
  event: '事件',
};

// Confidence level labels
export const CONFIDENCE_LABELS: Record<string, string> = {
  high: '高',
  medium: '中',
  low: '低',
};

export type AgentType = 'technique' | 'character' | 'setting' | 'event' | 'style';

export const AGENT_TYPE_LABELS: Record<AgentType, string> = {
  technique: '技法分析',
  character: '人物提取',
  setting: '设定提取',
  event: '事件提取',
  style: '风格分析',
};

export const AGENT_TYPES: { type: AgentType; label: string }[] = [
  { type: 'technique', label: '技法分析' },
  { type: 'character', label: '人物提取' },
  { type: 'setting', label: '设定提取' },
  { type: 'event', label: '事件提取' },
  { type: 'style', label: '风格分析' },
];

export type BuiltInAgentKind = 'technique_analysis' | 'character_extraction' | 'setting_extraction' | 'event_extraction' | 'style_analysis';

export const BUILT_IN_AGENT_LABELS: Record<BuiltInAgentKind, string> = {
  technique_analysis: '技法分析',
  character_extraction: '人物提取',
  setting_extraction: '设定提取',
  event_extraction: '事件提取',
  style_analysis: '风格分析',
};

export type ThemeMode = 'light' | 'dark' | 'system';

export const THEME_OPTIONS: { value: ThemeMode; label: string }[] = [
  { value: 'light', label: '浅色模式' },
  { value: 'dark', label: '深色模式' },
  { value: 'system', label: '跟随系统' },
];

// FabricSelect expects string values; keep these as strings and parse where needed.
export const RETRY_COUNT_OPTIONS: { value: string; label: string }[] = [
  { value: '0', label: '不重试' },
  { value: '1', label: '1 次' },
  { value: '2', label: '2 次' },
  { value: '3', label: '3 次（推荐）' },
  { value: '5', label: '5 次' },
];

export type AutoAcceptLevel = 'off' | 'high' | 'medium' | 'low';

export const AUTO_ACCEPT_OPTIONS: { value: AutoAcceptLevel; label: string }[] = [
  { value: 'off', label: '关闭（手动审核全部）' },
  { value: 'high', label: '仅高置信度' },
  { value: 'medium', label: '高+中置信度' },
  { value: 'low', label: '全部自动接受' },
];

/**
 * Format character role to Chinese
 */
export function formatRole(role: string | undefined | null): string {
  if (!role) return '角色';
  return ROLE_LABELS[role.toLowerCase()] || role;
}

/**
 * Format setting type to Chinese
 */
export function formatSettingType(type: string | undefined | null): string {
  if (!type) return '未分类';
  return SETTING_TYPE_LABELS[type.toLowerCase()] || type;
}

/**
 * Format event importance to Chinese
 */
export function formatImportance(importance: string | undefined | null): string {
  if (!importance) return '普通';
  return IMPORTANCE_LABELS[importance.toLowerCase()] || importance;
}

/**
 * Format knowledge type to Chinese
 */
export function formatKnowledgeType(type: string | undefined | null): string {
  if (!type) return type || '';
  return KNOWLEDGE_TYPE_LABELS[type.toLowerCase()] || type;
}

/**
 * Format confidence level to Chinese
 */
export function formatConfidence(confidence: string | undefined | null): string {
  if (!confidence) return confidence || '';
  return CONFIDENCE_LABELS[confidence.toLowerCase()] || confidence;
}
