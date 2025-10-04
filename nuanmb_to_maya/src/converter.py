"""
Main converter module for NUANMB to Maya animation format.
Orchestrates the conversion process from JSON to .anim file.
"""

from typing import List, Dict
from .nuanmb_parser import NuanmbParser
from .maya_writer import MayaAnimWriter
from .math_utils import quat_to_euler
from .models import (
    Node, Track, MayaAnimCurve, MayaKeyframe, GroupType
)


class NuanmbToMayaConverter:
    """Convert NUANMB animation to Maya .anim format"""
    
    def __init__(self, input_json: str, output_anim: str, 
                 maya_fps: float = 29.97, maya_version: str = "2020"):
        """
        Initialize converter.
        
        Args:
            input_json: Path to input NUANMB JSON file (from ssbh_data)
            output_anim: Path to output Maya .anim file
            maya_fps: Target Maya FPS (default: 29.97 for ntsc)
            maya_version: Maya version string (default: "2020")
        """
        self.input_json = input_json
        self.output_anim = output_anim
        self.maya_fps = maya_fps
        self.maya_version = maya_version
        
        # Calculate FPS conversion factor (NUANMB is 60fps)
        self.fps_conversion = maya_fps / 60.0
        
        # Determine time unit based on FPS
        self.time_unit = self._determine_time_unit(maya_fps)
        
        self.parser = NuanmbParser(input_json)
        self.writer = MayaAnimWriter(
            output_anim, 
            maya_version=maya_version,
            time_unit=self.time_unit,
            fps=maya_fps
        )
    
    def _determine_time_unit(self, fps: float) -> str:
        """
        Determine Maya time unit based on FPS.
        
        Args:
            fps: Frames per second
            
        Returns:
            Maya time unit string
        """
        fps_map = {
            15.0: "game",
            24.0: "film",
            25.0: "pal",
            29.97: "ntsc",
            30.0: "ntsc",
            48.0: "show",
            50.0: "palf",
            59.94: "ntscf",
            60.0: "ntscf"
        }
        
        # Find closest FPS match
        closest_fps = min(fps_map.keys(), key=lambda x: abs(x - fps))
        return fps_map.get(closest_fps, "ntsc")
    
    def convert(self):
        """
        Main conversion process.
        
        Raises:
            FileNotFoundError: If input file not found
            Exception: If conversion fails
        """
        print(f"Starting conversion: {self.input_json} -> {self.output_anim}")
        print(f"FPS conversion: 60fps (NUANMB) -> {self.maya_fps}fps (Maya {self.time_unit})")
        
        # Step 1: Parse NUANMB JSON
        print("Parsing NUANMB JSON...")
        anim_data = self.parser.parse()
        print(f"  Version: {anim_data.major_version}.{anim_data.minor_version}")
        print(f"  Final frame: {anim_data.final_frame_index}")
        print(f"  Groups: {len(anim_data.groups)}")
        
        # Step 2: Extract Transform groups only (bone animations)
        transform_groups = [g for g in anim_data.groups 
                          if g.group_type == GroupType.TRANSFORM]
        print(f"  Transform groups: {len(transform_groups)}")
        
        # Count bones
        bone_count = sum(len(g.nodes) for g in transform_groups)
        print(f"  Bones: {bone_count}")
        
        # Step 3: Process each bone
        print("Converting bone animations...")
        for group in transform_groups:
            for node in group.nodes:
                self._process_bone(node, anim_data.final_frame_index)
        
        # Step 4: Write Maya file
        print("Writing Maya .anim file...")
        self.writer.write()
        
        print(f"\nConversion complete!")
        print(f"  Output: {self.output_anim}")
        print(f"  Total curves: {self.writer.get_curve_count()}")
        print(f"  Total keyframes: {self.writer.get_keyframe_count()}")
    
    def _process_bone(self, node: Node, final_frame: float):
        """
        Process a single bone's animation and generate Maya curves.
        
        Args:
            node: Animation node (bone)
            final_frame: Final frame index of animation
        """
        bone_name = node.name
        
        # Find transform track
        transform_track = None
        for track in node.tracks:
            if track.name == "Transform" and len(track.values) > 0:
                transform_track = track
                break
        
        if not transform_track:
            return
        
        # Generate curves for each transform component
        curve_index = 0
        
        # Translation curves (X, Y, Z)
        for axis, attr in [('x', 'translateX'), ('y', 'translateY'), ('z', 'translateZ')]:
            keys = self._create_translation_keys(transform_track, axis, final_frame)
            if keys:
                curve = MayaAnimCurve(
                    attribute_path=f"translate.{attr}",
                    attribute_name=attr,
                    object_name=bone_name,
                    input_type=0,
                    output_type=0,  # Linear distance/translation
                    index=curve_index,
                    keys=keys
                )
                self.writer.add_curve(curve)
                curve_index += 1
        
        # Rotation curves (convert quaternion to Euler X, Y, Z)
        euler_keys = self._create_rotation_keys(transform_track, final_frame)
        
        for axis, attr in [('x', 'rotateX'), ('y', 'rotateY'), ('z', 'rotateZ')]:
            keys = euler_keys.get(axis, [])
            if keys:
                curve = MayaAnimCurve(
                    attribute_path=f"rotate.{attr}",
                    attribute_name=attr,
                    object_name=bone_name,
                    input_type=0,
                    output_type=1,  # Angular
                    index=curve_index,
                    keys=keys
                )
                self.writer.add_curve(curve)
                curve_index += 1
        
        # Scale curves (X, Y, Z)
        for axis, attr in [('x', 'scaleX'), ('y', 'scaleY'), ('z', 'scaleZ')]:
            keys = self._create_scale_keys(transform_track, axis, final_frame)
            if keys:
                curve = MayaAnimCurve(
                    attribute_path=f"scale.{attr}",
                    attribute_name=attr,
                    object_name=bone_name,
                    input_type=0,
                    output_type=0,  # Unitless
                    index=curve_index,
                    keys=keys
                )
                self.writer.add_curve(curve)
                curve_index += 1
    
    def _create_translation_keys(self, track: Track, axis: str, 
                                 final_frame: float) -> List[MayaKeyframe]:
        """
        Create translation keyframes for a specific axis.
        
        Args:
            track: Animation track
            axis: Axis name ('x', 'y', or 'z')
            final_frame: Final frame index
            
        Returns:
            List of Maya keyframes (without duplicates)
        """
        keys = []
        values = track.values
        last_maya_frame = -1
        
        for frame_idx, transform in enumerate(values):
            # Convert frame from 60fps to Maya fps
            maya_frame = int(frame_idx * self.fps_conversion)
            
            # Skip duplicate frames
            if maya_frame == last_maya_frame:
                continue
            
            last_maya_frame = maya_frame
            
            # Get value for the axis
            if axis == 'x':
                value = transform.translation.x
            elif axis == 'y':
                value = transform.translation.y
            else:
                value = transform.translation.z
            
            keys.append(MayaKeyframe(
                frame=maya_frame,
                value=value
            ))
        
        return keys
    
    def _create_rotation_keys(self, track: Track, final_frame: float) -> Dict[str, List[MayaKeyframe]]:
        """
        Create rotation keyframes (convert quaternion to Euler).
        
        Args:
            track: Animation track
            final_frame: Final frame index
            
        Returns:
            Dictionary mapping axis ('x', 'y', 'z') to keyframe lists (without duplicates)
        """
        euler_keys = {'x': [], 'y': [], 'z': []}
        values = track.values
        last_maya_frame = -1
        
        for frame_idx, transform in enumerate(values):
            # Convert frame from 60fps to Maya fps
            maya_frame = int(frame_idx * self.fps_conversion)
            
            # Skip duplicate frames
            if maya_frame == last_maya_frame:
                continue
            
            last_maya_frame = maya_frame
            
            # Convert quaternion to Euler angles (in degrees)
            euler = quat_to_euler(transform.rotation, order='XYZ')
            
            euler_keys['x'].append(MayaKeyframe(frame=maya_frame, value=euler.x))
            euler_keys['y'].append(MayaKeyframe(frame=maya_frame, value=euler.y))
            euler_keys['z'].append(MayaKeyframe(frame=maya_frame, value=euler.z))
        
        return euler_keys
    
    def _create_scale_keys(self, track: Track, axis: str, 
                          final_frame: float) -> List[MayaKeyframe]:
        """
        Create scale keyframes for a specific axis.
        
        Args:
            track: Animation track
            axis: Axis name ('x', 'y', or 'z')
            final_frame: Final frame index
            
        Returns:
            List of Maya keyframes (without duplicates)
        """
        keys = []
        values = track.values
        last_maya_frame = -1
        
        for frame_idx, transform in enumerate(values):
            # Convert frame from 60fps to Maya fps
            maya_frame = int(frame_idx * self.fps_conversion)
            
            # Skip duplicate frames
            if maya_frame == last_maya_frame:
                continue
            
            last_maya_frame = maya_frame
            
            # Get value for the axis
            if axis == 'x':
                value = transform.scale.x
            elif axis == 'y':
                value = transform.scale.y
            else:
                value = transform.scale.z
            
            keys.append(MayaKeyframe(
                frame=maya_frame,
                value=value
            ))
        
        return keys

