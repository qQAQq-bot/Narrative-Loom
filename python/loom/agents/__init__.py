from .base import BaseAgent, AgentConfig, AnalysisContext
from .technique import TechniqueAgent
from .character import CharacterAgent
from .setting import SettingAgent
from .event import EventAgent
from .style import StyleAgent

__all__ = [
    "BaseAgent",
    "AgentConfig",
    "AnalysisContext",
    "TechniqueAgent",
    "CharacterAgent",
    "SettingAgent",
    "EventAgent",
    "StyleAgent",
]


def create_agent(config: AgentConfig, provider) -> BaseAgent:
    agent_map = {
        "technique": TechniqueAgent,
        "character": CharacterAgent,
        "setting": SettingAgent,
        "event": EventAgent,
        "style": StyleAgent,
    }

    agent_class = agent_map.get(config.kind)
    if not agent_class:
        raise ValueError(f"Unknown agent kind: {config.kind}")

    return agent_class(config, provider)
