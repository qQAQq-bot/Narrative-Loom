"""
Pydantic models for analysis cards and related data structures.
These models provide type safety and validation for LLM analysis outputs.
"""

from typing import Optional, Any
from pydantic import BaseModel, Field


# =============================================================================
# Technique Card Models
# =============================================================================

class TechniqueItem(BaseModel):
    """A single writing technique identified in the chapter."""
    type: str = Field(..., description="Technique type (e.g., 'narrative', 'description')")
    title: str = Field(..., description="Technique name")
    description: str = Field(..., description="Technique description")
    mechanism: str = Field(..., description="How the technique works")
    evidence: list[str] = Field(default_factory=list, description="Text evidence from the chapter")
    tags: list[str] = Field(default_factory=list, description="Related tags")


class TechniqueAnalysisResult(BaseModel):
    """Result of technique analysis for a chapter."""
    techniques: list[TechniqueItem] = Field(default_factory=list)


# =============================================================================
# Character Card Models
# =============================================================================

class CharacterItem(BaseModel):
    """A character identified in the chapter."""
    name: str = Field(..., description="Character name")
    aliases: list[str] = Field(default_factory=list, description="Alternative names")
    description: Optional[str] = Field(None, description="Character description")
    traits: list[str] = Field(default_factory=list, description="Personality traits")
    role: str = Field("minor", description="Role type: protagonist/antagonist/supporting/minor")
    relationships: dict[str, Any] = Field(default_factory=dict, description="Relationships with other characters")
    evidence: list[str] = Field(default_factory=list, description="Text evidence from the chapter")
    is_new: bool = Field(False, description="Whether this is a new character in this chapter")


class CharacterAnalysisResult(BaseModel):
    """Result of character analysis for a chapter."""
    characters: list[CharacterItem] = Field(default_factory=list)


# =============================================================================
# Setting Card Models
# =============================================================================

class SettingItem(BaseModel):
    """A world setting element identified in the chapter."""
    type: str = Field(..., description="Setting type: location/era/worldview/item")
    name: str = Field(..., description="Setting name")
    description: Optional[str] = Field(None, description="Setting description")
    properties: dict[str, Any] = Field(default_factory=dict, description="Detailed properties")
    evidence: list[str] = Field(default_factory=list, description="Text evidence from the chapter")
    is_new: bool = Field(False, description="Whether this is a new setting in this chapter")


class SettingAnalysisResult(BaseModel):
    """Result of setting analysis for a chapter."""
    settings: list[SettingItem] = Field(default_factory=list)


# =============================================================================
# Event Card Models
# =============================================================================

class EventItem(BaseModel):
    """An important event identified in the chapter."""
    title: str = Field(..., description="Event title")
    description: str = Field(..., description="Event description")
    importance: str = Field("normal", description="Importance level: critical/major/normal/minor")
    characters_involved: list[str] = Field(default_factory=list, description="Characters involved in the event")
    evidence: list[str] = Field(default_factory=list, description="Text evidence from the chapter")
    is_turning_point: bool = Field(False, description="Whether this is a turning point")
    foreshadowing: bool = Field(False, description="Whether this involves foreshadowing")
    is_new: bool = Field(True, description="Whether this is a new event in this chapter")


class EventAnalysisResult(BaseModel):
    """Result of event analysis for a chapter."""
    events: list[EventItem] = Field(default_factory=list)


# =============================================================================
# Combined Analysis Result
# =============================================================================

class ChapterAnalysisResult(BaseModel):
    """Combined result of all analyses for a chapter."""
    techniques: list[TechniqueItem] = Field(default_factory=list)
    characters: list[CharacterItem] = Field(default_factory=list)
    settings: list[SettingItem] = Field(default_factory=list)
    events: list[EventItem] = Field(default_factory=list)


# =============================================================================
# Knowledge Card Models (for Story Bible)
# =============================================================================

class KnowledgeCard(BaseModel):
    """A knowledge card for the story bible."""
    knowledge_type: str = Field(..., description="Type: character/setting/event")
    title: str = Field(..., description="Card title")
    content: dict[str, Any] = Field(default_factory=dict, description="Card content")
    evidence: list[str] = Field(default_factory=list, description="Supporting evidence")
    confidence: str = Field("medium", description="Confidence level: high/medium/low")
    status: str = Field("pending", description="Status: pending/accepted/rejected")


# =============================================================================
# JSON Schema Generation Helpers
# =============================================================================

def get_technique_schema() -> dict:
    """Get JSON schema for technique analysis output."""
    return TechniqueAnalysisResult.model_json_schema()


def get_character_schema() -> dict:
    """Get JSON schema for character analysis output."""
    return CharacterAnalysisResult.model_json_schema()


def get_setting_schema() -> dict:
    """Get JSON schema for setting analysis output."""
    return SettingAnalysisResult.model_json_schema()


def get_event_schema() -> dict:
    """Get JSON schema for event analysis output."""
    return EventAnalysisResult.model_json_schema()
