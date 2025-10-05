"""
NUANMB to Maya Animation Converter
Main entry point for converting Super Smash Bros. Ultimate animation files to Maya format.

Usage:
    python main.py input.json skeleton.json output.anim [--fps 29.97] [--maya-version 2020]
"""

import argparse
import sys
from pathlib import Path
from src.converter import NuanmbToMayaConverter


def main():
    """Main program entry point"""
    parser = argparse.ArgumentParser(
        description='Convert NUANMB animation JSON to Maya .anim format',
        epilog='Example: python main.py animation.json skeleton.json animation.anim --fps 24'
    )
    
    parser.add_argument(
        'input',
        help='Input NUANMB JSON file (exported from ssbh_data)'
    )
    
    parser.add_argument(
        'skeleton',
        help='Input skeleton JSON file (NUSKTB format) for bone ordering'
    )
    
    parser.add_argument(
        'output',
        help='Output Maya .anim file'
    )
    
    parser.add_argument(
        '--fps',
        type=float,
        default=29.97,
        help='Target Maya FPS (default: 29.97 for ntsc). Common values: 24 (film), 29.97 (ntsc), 30, 60. Use 60 to keep original NUANMB framerate.'
    )
    
    parser.add_argument(
        '--no-fps-conversion',
        action='store_true',
        help='Keep original 60fps framerate without conversion (equivalent to --fps 60)'
    )
    
    parser.add_argument(
        '--maya-version',
        type=str,
        default='2020',
        help='Maya version string (default: 2020)'
    )
    
    parser.add_argument(
        '--verbose',
        action='store_true',
        help='Enable verbose output'
    )
    
    args = parser.parse_args()
    
    # Validate input file
    input_path = Path(args.input)
    if not input_path.exists():
        print(f"Error: Input file not found: {args.input}", file=sys.stderr)
        return 1
    
    if input_path.suffix.lower() != '.json':
        print(f"Warning: Input file doesn't have .json extension: {args.input}", file=sys.stderr)
    
    # Validate skeleton file
    skeleton_path = Path(args.skeleton)
    if not skeleton_path.exists():
        print(f"Error: Skeleton file not found: {args.skeleton}", file=sys.stderr)
        return 1
    
    if skeleton_path.suffix.lower() != '.json':
        print(f"Warning: Skeleton file doesn't have .json extension: {args.skeleton}", file=sys.stderr)
    
    # Validate output path
    output_path = Path(args.output)
    if output_path.suffix.lower() != '.anim':
        print(f"Warning: Output file doesn't have .anim extension: {args.output}", file=sys.stderr)
    
    # Create output directory if it doesn't exist
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    # Determine target FPS
    target_fps = 60.0 if args.no_fps_conversion else args.fps
    
    try:
        # Create converter
        converter = NuanmbToMayaConverter(
            input_json=str(input_path),
            skeleton_json=str(skeleton_path),
            output_anim=str(output_path),
            maya_fps=target_fps,
            maya_version=args.maya_version
        )
        
        # Run conversion
        converter.convert()
        
        return 0
        
    except FileNotFoundError as e:
        print(f"Error: File not found - {e}", file=sys.stderr)
        return 1
    
    except Exception as e:
        print(f"Error during conversion: {e}", file=sys.stderr)
        if args.verbose:
            import traceback
            traceback.print_exc()
        return 1


if __name__ == '__main__':
    sys.exit(main())

