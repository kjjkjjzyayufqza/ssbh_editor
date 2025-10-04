"""
Data models for NUANMB animation data and Maya animation curves.
"""

from dataclasses import dataclass
from typing import List, Optional
from enum import Enum


class GroupType(Enum):
    """Animation group types in NUANMB format"""
    TRANSFORM = "Transform"
    VISIBILITY = "Visibility"
    MATERIAL = "Material"


@dataclass
class Vector3:
    """3D vector for position, scale, etc."""
    x: float
    y: float
    z: float
    
    def to_list(self) -> List[float]:
        """Convert to list format"""
        return [self.x, self.y, self.z]


@dataclass
class Vector4:
    """4D vector for quaternion rotation, colors, etc."""
    x: float
    y: float
    z: float
    w: float
    
    def to_list(self) -> List[float]:
        """Convert to list format"""
        return [self.x, self.y, self.z, self.w]


@dataclass
class Transform:
    """Transform data for a single frame"""
    translation: Vector3
    rotation: Vector4  # Quaternion (x, y, z, w)
    scale: Vector3


@dataclass
class TransformFlags:
    """Flags for overriding transform components"""
    override_translation: bool = False
    override_rotation: bool = False
    override_scale: bool = False
    override_compensate_scale: bool = False


@dataclass
class Track:
    """Animation track containing frame-by-frame data"""
    name: str
    compensate_scale: bool
    transform_flags: TransformFlags
    values: List[Transform]  # Frame-by-frame transform data


@dataclass
class Node:
    """Animation node (typically represents a bone)"""
    name: str  # Bone name
    tracks: List[Track]


@dataclass
class Group:
    """Animation group containing multiple nodes"""
    group_type: GroupType
    nodes: List[Node]


@dataclass
class AnimData:
    """Root animation data structure"""
    major_version: int
    minor_version: int
    final_frame_index: float
    groups: List[Group]


@dataclass
class MayaKeyframe:
    """Single keyframe in Maya animation curve"""
    frame: int
    value: float
    in_tangent: str = "auto"
    out_tangent: str = "auto"
    lock: int = 1
    weight_lock: int = 0 # Match working example.anim format
    breakdown: int = 0


@dataclass
class MayaAnimCurve:
    """Maya animation curve definition"""
    attribute_path: str      # e.g., "translate.translateX"
    attribute_name: str      # e.g., "translateX"
    object_name: str         # Bone/object name
    input_type: int          # 0 = time
    output_type: int         # 1 = angular, 2 = linear
    index: int               # Curve index
    keys: List[MayaKeyframe]
