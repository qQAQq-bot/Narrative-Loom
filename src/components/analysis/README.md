# Analysis Components

Components for chapter analysis functionality.

## Components

### `BatchAnalysisModal.vue`
Modal dialog for batch chapter analysis with real-time progress tracking.

Features:
- Displays selected chapters before analysis starts
- Real-time progress updates via Tauri events
- Shows current chapter title and agent type being processed
- Progress bar with percentage
- Completed/failed chapter summary after completion
- Cancel/Start/Complete buttons based on state
- **Cannot be closed while analysis is running** (locked modal)
- Lock icon displayed in header during analysis

Props:
- `show: boolean` - Controls modal visibility
- `progress: BatchProgress` - Progress state object
- `selectedChapters: { id: string; title: string }[]` - Selected chapters info

Events:
- `close` - Close the modal (only works when not running)
- `cancel` - Cancel ongoing analysis
- `start` - Start batch analysis

### Real-time Progress Communication

The batch analysis uses Tauri events for real-time progress updates:

1. **Backend** (`analysis.rs`) emits `batch-analysis-progress` events containing:
   - `current_chapter_id`: ID of chapter being processed
   - `current_chapter_index`: 0-based index
   - `total_chapters`: Total count
   - `current_agent_type`: Current analysis agent (technique/character/setting/event)
   - `status`: "running", "completed", "error", "cancelled"

2. **Frontend** (`analysis.ts` store) listens to these events and updates `batchProgress` state

3. **Modal** reactively displays the updated progress information

### Sidebar Integration

The batch analysis button in `Sidebar.vue`:
- Shows "批量分析" when idle with chapters selected
- Shows "查看进度" with amber background when analysis is running
- Has a pulsing red indicator dot when analysis is running
- Can be clicked to reopen the progress modal at any time during analysis

## Recent Changes

### v0.3.0 - Real-time Batch Analysis Progress
- Added Tauri event listener for `batch-analysis-progress` events
- Real-time display of current chapter title and agent type
- Improved status text with chapter count display
- Added `useTauri.listen()` for event subscription support
- **Modal locking**: Cannot close modal while analysis is running
- **Progress persistence**: Clicking the button reopens modal with current progress
- **Visual indicators**: Amber button color and pulsing dot when running
