"""
Parser for NUANMB animation data in JSON format.
Converts JSON exported by ssbh_data into Python data structures.
"""

import json
from typing import List, Dict, Any, Optional
from .models import (
    AnimData, Group, Node, Track, Transform, TransformFlags,
    Vector3, Vector4, GroupType
)


class NuanmbParser:
    """Parse NUANMB JSON data exported by ssbh_data"""
    
    def __init__(self, json_path: str):
        """
        Initialize parser with JSON file path.
        
        Args:
            json_path: Path to NUANMB JSON file
        """
        self.json_path = json_path
        self.anim_data: Optional[AnimData] = None
    
    def parse(self) -> AnimData:
        """
        Parse JSON file and return AnimData structure.
        
        Returns:
            Parsed animation data
            
        Raises:
            FileNotFoundError: If JSON file not found
            json.JSONDecodeError: If JSON is invalid
            KeyError: If required fields are missing
        """
        with open(self.json_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        self.anim_data = self._parse_anim_data(data)
        return self.anim_data
    
    def _parse_anim_data(self, data: Dict[str, Any]) -> AnimData:
        """Parse root AnimData structure"""
        return AnimData(
            major_version=data['major_version'],
            minor_version=data['minor_version'],
            final_frame_index=data['final_frame_index'],
            groups=[self._parse_group(g) for g in data['groups']]
        )
    
    def _parse_group(self, data: Dict[str, Any]) -> Group:
        """Parse Group structure"""
        return Group(
            group_type=GroupType(data['group_type']),
            nodes=[self._parse_node(n) for n in data['nodes']]
        )
    
    def _parse_node(self, data: Dict[str, Any]) -> Node:
        """Parse Node structure (bone)"""
        return Node(
            name=data['name'],
            tracks=[self._parse_track(t) for t in data['tracks']]
        )
    
    def _parse_track(self, data: Dict[str, Any]) -> Track:
        """Parse Track structure"""
        # Parse values based on type
        values_data = data['values']
        
        if 'Transform' in values_data:
            values = [self._parse_transform(t) for t in values_data['Transform']]
        else:
            # Handle other value types if needed (Boolean, Float, etc.)
            # For now, just return empty list
            values = []
        
        return Track(
            name=data['name'],
            compensate_scale=data['compensate_scale'],
            transform_flags=self._parse_transform_flags(data['transform_flags']),
            values=values
        )
    
    def _parse_transform(self, data: Dict[str, Any]) -> Transform:
        """Parse Transform structure"""
        return Transform(
            translation=Vector3(**data['translation']),
            rotation=Vector4(**data['rotation']),
            scale=Vector3(**data['scale'])
        )
    
    def _parse_transform_flags(self, data: Dict[str, Any]) -> TransformFlags:
        """Parse TransformFlags structure"""
        return TransformFlags(**data)
    
    def get_transform_groups(self) -> List[Group]:
        """
        Get only Transform type groups (bone animations).
        
        Returns:
            List of Transform groups
        """
        if not self.anim_data:
            return []
        
        return [g for g in self.anim_data.groups 
                if g.group_type == GroupType.TRANSFORM]
    
    def get_bone_names(self) -> List[str]:
        """
        Get list of all bone names in the animation.
        
        Returns:
            List of bone names
        """
        if not self.anim_data:
            return []
        
        bone_names = []
        for group in self.get_transform_groups():
            for node in group.nodes:
                bone_names.append(node.name)
        
        return bone_names

