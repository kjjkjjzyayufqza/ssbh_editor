"""
Main converter module for NUANMB to Maya animation format.
Orchestrates the conversion process from JSON to .anim file.
"""

from typing import List, Dict
import numpy as np
from .nuanmb_parser import NuanmbParser
from .maya_writer import MayaAnimWriter
from .math_utils import quat_to_euler, build_matrix4x4, matrix_to_trans_quat_scale
from .models import (
    Node, Track, MayaAnimCurve, MayaKeyframe, GroupType, Vector3, Transform
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

        # Apply root bone correction if bone is 'Trans'
        processed_values = transform_track.values
        if bone_name == "Trans":
            processed_values = [self._apply_root_correction(t) for t in transform_track.values]

        # Use a new Track object with corrected values for key creation functions
        transform_track_for_keys = Track(
             name=transform_track.name,
             values=processed_values,
             transform_flags=transform_track.transform_flags,
             compensate_scale=transform_track.compensate_scale
        )

        
        # Generate curves for each transform component
        curve_index = 0
        
        # Translation curves (X, Y, Z)
        for axis, attr in [('x', 'translateX'), ('y', 'translateY'), ('z', 'translateZ')]:
            keys = self._create_translation_keys(transform_track_for_keys, axis, final_frame)
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
        euler_keys = self._create_rotation_keys(transform_track_for_keys, final_frame)
        
        for axis, attr in [('x', 'rotateX'), ('y', 'rotateY'), ('z', 'rotateZ')]:
            keys = euler_keys.get(axis, [])
            if keys:
                curve = MayaAnimCurve(
                    attribute_path=f"rotate.{attr}",
                    attribute_name=attr,
                    object_name=bone_name,
                    input_type=0,
                    output_type=0,  # Use 0 to match reference file structure, rely on writer for text output
                    index=curve_index,
                    keys=keys
                )
                self.writer.add_curve(curve)
                curve_index += 1
        
        # Scale curves (X, Y, Z)
        for axis, attr in [('x', 'scaleX'), ('y', 'scaleY'), ('z', 'scaleZ')]:
            keys = self._create_scale_keys(transform_track_for_keys, axis, final_frame)
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
    
    def _apply_root_correction(self, raw_transform: Transform) -> Transform:
        """
        Applies world space correction matrices used by smash-ultimate-blender for the root bone.
        M_corr = R_X_90 @ M_SSBH @ R_Z_-90
        
        Args:
            raw_transform: The original SSBH Transform (T, R, S)
            
        Returns:
            The corrected Transform (T', R', S')
        """
        T = raw_transform.translation
        R = raw_transform.rotation
        S = raw_transform.scale
        
        # 1. Build the SSBH transformation matrix (T*R*S. Note: build_matrix4x4 uses T in column 4)
        M_ssbh = build_matrix4x4(T, R, S)
        
        # 2. Define the correction matrices R_X_90 and R_Z_-90 (Blender style, numpy array)
        # R_X_90 (Rotate 90 degrees around X)
        R_X_90 = np.array([
            [1, 0,  0, 0],
            [0, 0, -1, 0],
            [0, 1,  0, 0],
            [0, 0,  0, 1]
        ])
        
        # R_Z_-90 (Rotate -90 degrees around Z)
        R_Z_N90 = np.array([
            [0, 1, 0, 0],
            [-1, 0, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, 1]
        ])
        
        # 3. Calculate corrected matrix: M_corr = R_X_90 @ M_ssbh @ R_Z_-90
        M_corr = R_X_90 @ M_ssbh @ R_Z_N90
        
        # 4. Decompose back to T, R, S
        T_new, Q_new, S_new = matrix_to_trans_quat_scale(M_corr)
        
        return Transform(translation=T_new, rotation=Q_new, scale=S_new)


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
            # Coordinate System Transformation applied (Standard Z-up to Y-up): X_new=X_raw, Y_new=Z_raw, Z_new=-Y_raw
            if axis == 'x':
                # Map to raw X
                value = transform.translation.x
            elif axis == 'y':
                # Map to raw Z
                value = transform.translation.z
            else: # axis == 'z'
                # Map to raw -Y
                value = -transform.translation.y
            
            keys.append(MayaKeyframe(
                frame=maya_frame,
                value=value
            ))
        
        return keys
    
    def _create_rotation_keys(self, track: Track, final_frame: float) -> Dict[str, List[MayaKeyframe]]:
        """
        Create rotation keyframes (convert quaternion to Euler) with continuity correction.
        
        Args:
            track: Animation track
            final_frame: Final frame index
            
        Returns:
            Dictionary mapping axis ('x', 'y', 'z') to keyframe lists (without duplicates)
        """
        
        def _correct_rotation_winding(current: float, previous: float) -> float:
            """Adjusts current rotation value to maintain continuity relative to previous value."""
            # Correct rotation winding to ensure smooth curve (minimal 360 degree difference)
            while current - previous > 180.0:
                current -= 360.0
            while current - previous < -180.0:
                current += 360.0
            return current

        euler_keys = {'x': [], 'y': [], 'z': []}
        values = track.values
        last_maya_frame = -1
        
        # Track previous Euler angles for continuity
        prev_euler_x, prev_euler_y, prev_euler_z = 0.0, 0.0, 0.0
        
        for frame_idx, transform in enumerate(values):
            # Convert frame from 60fps to Maya fps
            maya_frame = int(frame_idx * self.fps_conversion)
            
            # Skip duplicate frames
            if maya_frame == last_maya_frame:
                continue
            
            last_maya_frame = maya_frame
            
            # Convert quaternion to Euler angles (in degrees)
            raw_euler = quat_to_euler(transform.rotation, order='XYZ')
            
            # Apply Coordinate System Transformation (Standard Z-up to Y-up mapping): X_new=X_raw, Y_new=Z_raw, Z_new=-Y_raw
            # Use computed Euler angles (raw_euler.x, raw_euler.y, raw_euler.z) and map them to Maya's XYZ axes.
            euler = Vector3(
                x=raw_euler.x,
                y=raw_euler.z,
                z=-raw_euler.y
            )
            
            # Apply continuity correction relative to the previous frame's stored value
            if frame_idx > 0:
                euler.x = _correct_rotation_winding(euler.x, prev_euler_x)
                euler.y = _correct_rotation_winding(euler.y, prev_euler_y)
                euler.z = _correct_rotation_winding(euler.z, prev_euler_z)
            
            # Update previous Euler angles for the next iteration
            prev_euler_x, prev_euler_y, prev_euler_z = euler.x, euler.y, euler.z
            
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
            # Coordinate System Transformation applied (Standard Z-up to Y-up mapping): X_new=X_raw, Y_new=Z_raw, Z_new=Y_raw
            if axis == 'x':
                # Map to raw X
                value = transform.scale.x
            elif axis == 'y':
                # Map to raw Z
                value = transform.scale.z
            else: # axis == 'z'
                # Map to raw Y
                value = transform.scale.y
            
            keys.append(MayaKeyframe(
                frame=maya_frame,
                value=value
            ))
        
        return keys
