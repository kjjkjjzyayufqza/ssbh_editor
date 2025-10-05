"""
Writer for Maya .anim file format.
Generates ASCII Maya animation files from animation curve data.
"""

from typing import List, Tuple, TextIO, Union, Any
from .models import MayaAnimCurve, MayaKeyframe


class MayaAnimWriter:
    """Write Maya .anim files in ASCII format"""
    
    def __init__(self, output_path: str, maya_version: str = "2020", 
                 time_unit: str = "ntsc", fps: float = 29.97):
        """
        Initialize Maya animation file writer.
        
        Args:
            output_path: Path to output .anim file
            maya_version: Maya version string (default: "2020")
            time_unit: Time unit (film=24fps, ntsc=29.97fps, pal=25fps, etc.)
            fps: Frames per second for the time unit
        """
        self.output_path = output_path
        self.maya_version = maya_version
        self.time_unit = time_unit
        self.fps = fps
        self.curves: List[MayaAnimCurve] = []
        self.empty_bones: List[str] = []
        self.write_order: List[Tuple[str, Union[MayaAnimCurve, str]]] = []  # ('curve', curve_obj) or ('empty', bone_name)
    
    def add_curve(self, curve: MayaAnimCurve):
        """
        Add an animation curve to be written.
        
        Args:
            curve: Maya animation curve
        """
        self.curves.append(curve)
        self.write_order.append(('curve', curve))
    
    def add_empty_bone(self, bone_name: str):
        """
        Add an empty bone entry (bone with no animation data).
        
        Args:
            bone_name: Name of the bone
        """
        self.empty_bones.append(bone_name)
        self.write_order.append(('empty', bone_name))
    
    def write(self):
        """
        Write all curves and empty bone entries to Maya .anim file in order.
        
        Raises:
            IOError: If file cannot be written
        """
        with open(self.output_path, 'w', encoding='utf-8') as f:
            self._write_header(f)
            
            # Write curves and empty bones in the order they were added
            for item_type, item_data in self.write_order:
                if item_type == 'curve':
                    self._write_curve(f, item_data)
                elif item_type == 'empty':
                    self._write_node_definition(f, item_data)
    
    def _write_header(self, f: TextIO):
        """Write Maya anim file header"""
        f.write("animVersion 1.1;\n")
        f.write(f"mayaVersion {self.maya_version};\n")
        f.write(f"timeUnit {self.time_unit};\n")
        f.write("linearUnit cm;\n")
        f.write("angularUnit deg;\n")
        
        # Calculate and write start/end time from all curves
        start_time, end_time = self._calculate_time_range()
        f.write(f"startTime {start_time};\n")
        f.write(f"endTime {end_time};\n")
    
    def _write_node_definition(self, f: TextIO, node_name: str):
        """
        Write a node definition line.
        
        Args:
            f: File handle
            node_name: Name of the node
        """
        f.write(f"anim {node_name} 0 1 0;\n")
    
    def _write_curve(self, f: TextIO, curve: MayaAnimCurve):
        """
        Write a single animation curve.
        
        Args:
            f: File handle
            curve: Animation curve to write
        """
        # Write curve header
        f.write(f"anim {curve.attribute_path} {curve.attribute_name} ")
        f.write(f"{curve.object_name} {curve.input_type} {curve.output_type} {curve.index};\n")
        # Write animData block
        f.write("animData {\n")
        f.write("  input time;\n")
        
        # Output type text is determined by the attribute path based on Maya conventions
        output_type_text = "unitless"
        if "rotate" in curve.attribute_path:
            output_type_text = "angular"
        elif "translate" in curve.attribute_path:
            output_type_text = "linear"
        # If not rotate or translate, keep as "unitless" (for scale and others)
        
        f.write(f"  output {output_type_text};\n")
        
        f.write("  weighted 0;\n")
        f.write("  preInfinity constant;\n")
        f.write("  postInfinity constant;\n")
        
        # Write keys
        if curve.keys:
            f.write("  keys {\n")
            for key in curve.keys:
                self._write_key(f, key)
            f.write("  }\n")
        
        f.write("}\n")
    
    def _write_key(self, f: TextIO, key: MayaKeyframe):
        """
        Write a single keyframe.
        
        Args:
            f: File handle
            key: Keyframe to write
        """
        f.write(f"    {key.frame} {key.value} ")
        f.write(f"{key.in_tangent} {key.out_tangent} ")
        f.write(f"{key.lock} {key.weight_lock} {key.breakdown};\n")
    
    def clear_curves(self):
        """Clear all curves from the writer"""
        self.curves.clear()
    
    def get_curve_count(self) -> int:
        """
        Get number of curves to be written.
        
        Returns:
            Number of curves
        """
        return len(self.curves)
    
    def get_keyframe_count(self) -> int:
        """
        Get total number of keyframes across all curves.
        
        Returns:
            Total keyframe count
        """
        return sum(len(curve.keys) for curve in self.curves)
    
    def _calculate_time_range(self) -> Tuple[int, int]:
        """
        Calculate the time range (start and end frames) from all curves.
        
        Returns:
            Tuple of (start_time, end_time)
        """
        if not self.curves:
            return (0, 0)
        
        min_frame = float('inf')
        max_frame = float('-inf')
        
        for curve in self.curves:
            if curve.keys:
                frames = [key.frame for key in curve.keys]
                min_frame = min(min_frame, min(frames))
                max_frame = max(max_frame, max(frames))
        
        # If no valid frames found, default to 0-1
        if min_frame == float('inf') or max_frame == float('-inf'):
            return (0, 1)
        
        return (int(min_frame), int(max_frame))
