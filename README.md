# Tauri + Vue 3

This template should help get you started developing with Tauri + Vue 3 in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## UI Notes

- Settings page: `src/views/SettingsView.vue`
- Shared select component: `src/components/ui/FabricSelect.vue`
  - Width is controlled by the parent container (e.g. `w-40`)
  - Long option labels are truncated with ellipsis inside the button

## System Prompt Cards

System Prompt Cards allow you to inject global prompts to all AI agents without modifying individual agent configurations.

### How it works

Cards are applied in order:
```
[Prefix Card 1] + [Prefix Card 2] + ...
+ [Agent's built-in system prompt]
+ [Suffix Card 1] + [Suffix Card 2] + ...
```

### Configuration

- Cards are stored in `prompt_cards.json` in the config directory
- Manage cards via Settings > Prompt Cards in the UI
- Each card has:
  - **Title**: Display name
  - **Content**: The prompt text to inject
  - **Position**: `prefix` (before agent prompt) or `suffix` (after agent prompt)
  - **Enabled**: Toggle on/off
  - **Order**: Ordering within the same position

### Use Cases

- Add disclaimers for content moderation (e.g., "This is fictional content analysis")
- Inject global formatting instructions
- Add consistent output requirements across all agents

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Entity Recall Validation

Use the following commands inside `src-tauri` to verify entity-recall related changes:

```bash
cargo test retrieval:: -- --nocapture
cargo test entity_recall_context_integration -- --nocapture
cargo test entity_recall_metrics -- --nocapture
cargo check
```
