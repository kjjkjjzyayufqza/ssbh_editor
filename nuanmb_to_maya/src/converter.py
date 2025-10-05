"""
Main converter module for NUANMB to Maya animation format.
Orchestrates the conversion process from JSON to .anim file.
"""

from typing import List, Dict, Tuple
import json
import numpy as np
from .nuanmb_parser import NuanmbParser
from .maya_writer import MayaAnimWriter
from .math_utils import (quat_to_euler, build_matrix4x4, matrix_to_trans_quat_scale, 
                         quat_multiply, axis_angle_to_quat)
from .models import (
    Node, Track, MayaAnimCurve, MayaKeyframe, GroupType, Vector3, Transform
)


class NuanmbToMayaConverter:
    """Convert NUANMB animation to Maya .anim format"""
    
    def __init__(self, input_json: str, skeleton_json: str, output_anim: str, 
                 maya_fps: float = 29.97, maya_version: str = "2020"):
        """
        Initialize converter.
        
        Args:
            input_json: Path to input NUANMB JSON file (from ssbh_data)
            skeleton_json: Path to skeleton JSON file (NUSKTB format) for bone ordering
            output_anim: Path to output Maya .anim file
            maya_fps: Target Maya FPS (default: 29.97 for ntsc)
            maya_version: Maya version string (default: "2020")
        """
        self.input_json = input_json
        self.skeleton_json = skeleton_json
        self.output_anim = output_anim
        self.maya_fps = maya_fps
        self.maya_version = maya_version
        
        # Calculate FPS conversion factor (NUANMB is 60fps)
        self.fps_conversion = maya_fps / 60.0
        
        # Determine time unit based on FPS
        self.time_unit = self._determine_time_unit(maya_fps)
        
        # Load skeleton bone order
        self.bone_order = self._load_skeleton_bone_order()
        
        self.parser = NuanmbParser(input_json)
        self.writer = MayaAnimWriter(
            output_anim, 
            maya_version=maya_version,
            time_unit=self.time_unit,
            fps=maya_fps
        )
    
    def _load_skeleton_bone_order(self) -> List[str]:
        """
        Load bone order from skeleton JSON file and sort by hierarchy.
        Uses parent_index to ensure parents are always before children.
        
        Returns:
            List of bone names in hierarchical order (parents before children)
            
        Raises:
            FileNotFoundError: If skeleton file not found
            Exception: If skeleton file parsing fails
        """
        try:
            with open(self.skeleton_json, 'r', encoding='utf-8') as f:
                skeleton_data = json.load(f)
            
            if 'bones' not in skeleton_data:
                return []
            
            bones = skeleton_data['bones']
            print(f"Loaded skeleton: {len(bones)} bones")
            
            # Perform topological sort based on parent_index
            bone_order = self._topological_sort_bones(bones)
            print(f"Sorted bones by hierarchy: {len(bone_order)} bones")
            
            return bone_order
            
        except FileNotFoundError:
            raise FileNotFoundError(f"Skeleton file not found: {self.skeleton_json}")
        except Exception as e:
            raise Exception(f"Failed to parse skeleton file: {e}")
    
    def _topological_sort_bones(self, bones: List[Dict]) -> List[str]:
        """
        Sort bones by hierarchy using topological sort.
        Ensures parent bones are always placed before their children.
        
        Args:
            bones: List of bone dictionaries with 'name' and 'parent_index' fields
            
        Returns:
            List of bone names sorted by hierarchy
        """
        # Build adjacency list (parent -> children mapping)
        children_map = {}
        bone_names = []
        root_indices = []
        
        for i, bone in enumerate(bones):
            bone_name = bone.get('name', f'bone_{i}')
            bone_names.append(bone_name)
            parent_idx = bone.get('parent_index')
            
            if parent_idx is None:
                root_indices.append(i)
            else:
                if parent_idx not in children_map:
                    children_map[parent_idx] = []
                children_map[parent_idx].append(i)
        
        # Perform depth-first traversal from root bones
        sorted_order = []
        visited = set()
        
        def dfs(bone_index: int):
            """Depth-first search to traverse bone hierarchy"""
            if bone_index in visited:
                return
            
            visited.add(bone_index)
            sorted_order.append(bone_names[bone_index])
            
            # Visit children
            if bone_index in children_map:
                for child_idx in children_map[bone_index]:
                    dfs(child_idx)
        
        # Start DFS from all root bones
        for root_idx in root_indices:
            dfs(root_idx)
        
        # Add any unvisited bones (shouldn't happen in valid skeleton)
        for i, bone_name in enumerate(bone_names):
            if i not in visited:
                print(f"Warning: Orphan bone found: {bone_name}")
                sorted_order.append(bone_name)
        
        return sorted_order
    
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
        
        # Step 3: Collect all nodes from transform groups
        all_nodes = []
        for group in transform_groups:
            all_nodes.extend(group.nodes)
        
        bone_count = len(all_nodes)
        print(f"  Bones: {bone_count}")
        
        # Step 4: Create bone name to node mapping
        node_map = {node.name: node for node in all_nodes}
        
        # Step 5: Process bones in skeleton order, adding empty entries for missing bones
        print("Converting bone animations...")
        bones_with_anim = 0
        bones_without_anim = 0
        
        for bone_name in self.bone_order:
            if bone_name in node_map:
                # Bone has animation data
                self._process_bone(node_map[bone_name], anim_data.final_frame_index)
                bones_with_anim += 1
            else:
                # Bone exists in skeleton but not in animation - add empty entry
                self.writer.add_empty_bone(bone_name)
                bones_without_anim += 1
        
        # Process any bones in animation but not in skeleton (at the end)
        for node in all_nodes:
            if node.name not in self.bone_order:
                print(f"Warning: Bone '{node.name}' found in animation but not in skeleton")
                self._process_bone(node, anim_data.final_frame_index)
                bones_with_anim += 1
        
        print(f"  Bones with animation: {bones_with_anim}")
        print(f"  Bones without animation (empty): {bones_without_anim}")
        
        # Step 6: Write Maya file
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
        is_root_bone = (bone_name == "Trans")
        
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
        if is_root_bone:
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
            keys = self._create_translation_keys(transform_track_for_keys, axis, final_frame, is_root_bone)
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
        euler_keys = self._create_rotation_keys(transform_track_for_keys, final_frame, is_root_bone)
        
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
            keys = self._create_scale_keys(transform_track_for_keys, axis, final_frame, is_root_bone)
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
        Applies world space correction used by smash-ultimate-blender for the root bone.
        Uses quaternion multiplication for rotation: Q_corr = Q_X_90 * Q_raw * Q_Z_-90
        
        The coordinate transformation converts from SSBH's Z-up right-handed system
        to Maya's Y-up right-handed system.
        
        Args:
            raw_transform: The original SSBH Transform (T, R, S)
            
        Returns:
            The corrected Transform (T', R', S')
        """
        T = raw_transform.translation
        R = raw_transform.rotation
        S = raw_transform.scale
        
        # Create rotation quaternions for coordinate system conversion
        # Q_X_90: Rotate 90 degrees around X axis (Z-up to Y-up)
        Q_X_90 = axis_angle_to_quat(Vector3(x=1.0, y=0.0, z=0.0), 90.0)
        
        # Q_Z_-90: Rotate -90 degrees around Z axis (X-major to Y-major)
        Q_Z_N90 = axis_angle_to_quat(Vector3(x=0.0, y=0.0, z=1.0), -90.0)
        
        # Apply rotation correction: Q_corr = Q_X_90 * Q_raw * Q_Z_-90
        Q_temp = quat_multiply(R, Q_Z_N90)
        Q_corr = quat_multiply(Q_X_90, Q_temp)
        
        # Transform translation using the same logic
        # Apply transformation matrix approach for translation
        M_ssbh = build_matrix4x4(T, R, S)
        
        R_X_90 = np.array([
            [1, 0,  0, 0],
            [0, 0, -1, 0],
            [0, 1,  0, 0],
            [0, 0,  0, 1]
        ])
        
        R_Z_N90 = np.array([
            [0, 1, 0, 0],
            [-1, 0, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, 1]
        ])
        
        M_corr = R_X_90 @ M_ssbh @ R_Z_N90
        T_corr = Vector3(x=M_corr[0, 3], y=M_corr[1, 3], z=M_corr[2, 3])
        
        # Scale remains the same (coordinate system change doesn't affect scale)
        S_corr = S
        
        return Transform(translation=T_corr, rotation=Q_corr, scale=S_corr)


    def _create_translation_keys(self, track: Track, axis: str, 
                                 final_frame: float, is_root_bone: bool = False) -> List[MayaKeyframe]:
        """
        Create translation keyframes for a specific axis.
        
        Args:
            track: Animation track
            axis: Axis name ('x', 'y', or 'z')
            final_frame: Final frame index
            is_root_bone: Whether this is the root bone (Trans)
            
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
            if is_root_bone:
                # Root bone has already been transformed by _apply_root_correction
                # No additional coordinate mapping needed
                if axis == 'x':
                    value = transform.translation.x
                elif axis == 'y':
                    value = transform.translation.y
                else:  # axis == 'z'
                    value = transform.translation.z
            else:
                # Non-root bones: For Maya/Blender compatibility,  
                # bones might need different handling based on the bone axis convention
                # For now, use the same Z-up to Y-up mapping as root bone
                # TODO: May need to apply bone-local coordinate transformation
                if axis == 'x':
                    value = transform.translation.x
                elif axis == 'y':
                    value = transform.translation.z
                else:  # axis == 'z'
                    value = -transform.translation.y
            
            keys.append(MayaKeyframe(
                frame=maya_frame,
                value=value
            ))
        
        return keys
    
    def _create_rotation_keys(self, track: Track, final_frame: float, is_root_bone: bool = False) -> Dict[str, List[MayaKeyframe]]:
        """
        Create rotation keyframes (convert quaternion to Euler) with continuity correction.
        
        Args:
            track: Animation track
            final_frame: Final frame index
            is_root_bone: Whether this is the root bone (Trans)
            
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
            
            if is_root_bone:
                # Root bone has already been transformed by _apply_root_correction
                # No additional coordinate mapping needed
                euler = Vector3(
                    x=raw_euler.x,
                    y=raw_euler.y,
                    z=raw_euler.z
                )
            else:
                # Non-root bones need coordinate system transformation (Z-up to Y-up)
                # X_new=X_raw, Y_new=Z_raw, Z_new=-Y_raw
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
                          final_frame: float, is_root_bone: bool = False) -> List[MayaKeyframe]:
        """
        Create scale keyframes for a specific axis.
        
        Args:
            track: Animation track
            axis: Axis name ('x', 'y', or 'z')
            final_frame: Final frame index
            is_root_bone: Whether this is the root bone (Trans)
            
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
            if is_root_bone:
                # Root bone has already been transformed by _apply_root_correction
                # No additional coordinate mapping needed
                if axis == 'x':
                    value = transform.scale.x
                elif axis == 'y':
                    value = transform.scale.y
                else:  # axis == 'z'
                    value = transform.scale.z
            else:
                # Non-root bones need coordinate system transformation (Z-up to Y-up)
                # X_new=X_raw, Y_new=Z_raw, Z_new=Y_raw (scale doesn't flip sign)
                if axis == 'x':
                    value = transform.scale.x
                elif axis == 'y':
                    value = transform.scale.z
                else:  # axis == 'z'
                    value = transform.scale.y
            
            keys.append(MayaKeyframe(
                frame=maya_frame,
                value=value
            ))
        
        return keys
