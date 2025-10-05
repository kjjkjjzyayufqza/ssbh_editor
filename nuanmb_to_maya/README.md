# NUANMB to Maya Animation Converter

Convert Super Smash Bros. Ultimate `.nuanmb` animation files to Maya `.anim` format.

## Overview

This tool converts NUANMB (Nintendo U Animation Binary) animation files from Super Smash Bros. Ultimate into Maya-compatible ASCII animation files. The conversion workflow is:

```
.nuanmb → [ssbh_data] → .json → [This Tool] → .anim (Maya)
```

## Features

- ✅ Convert bone animations (translation, rotation, scale)
- ✅ Hierarchical bone ordering based on skeleton file (NUSKTB format)
- ✅ Topological sort ensuring parents are always before children
- ✅ Automatic empty bone entries for bones in skeleton but not in animation
- ✅ Quaternion to Euler angle conversion
- ✅ FPS conversion (60fps NUANMB to Maya FPS)
- ✅ Support for multiple Maya FPS standards (24, 29.97, 30, 60)
- ✅ Preserve animation timing and interpolation
- ✅ Clean Maya-compatible ASCII output

## Requirements

- Python 3.8 or higher
- numpy >= 1.21.0
- ssbh_data tool (for converting .nuanmb to .json)

## Installation

### Option 1: Direct Installation

```bash
pip install -r requirements.txt
```

### Option 2: Development Installation

```bash
pip install -e .
```

## Usage

### Step 1: Convert NUANMB and NUSKTB to JSON

First, use the `ssbh_data` tool to export the NUANMB and NUSKTB files to JSON:

```bash
# Export animation file
ssbh_data_json animation.nuanmb animation.json

# Export skeleton file (for bone ordering)
ssbh_data_json model.nusktb skeleton.json
```

### Step 2: Convert JSON to Maya Anim

**Recommended (keep 60fps with skeleton bone order):**
```bash
python main.py animation.json skeleton.json animation.anim --fps 60
```

**Or convert to Maya standard FPS:**
```bash
python main.py animation.json skeleton.json animation.anim --fps 29.97
```

### Advanced Options

```bash
# Keep original 60fps without conversion (recommended)
python main.py animation.json skeleton.json animation.anim --no-fps-conversion

# Specify target FPS
python main.py animation.json skeleton.json animation.anim --fps 24

# Specify Maya version (default: 2020)
python main.py animation.json skeleton.json animation.anim --maya-version 2023

# Enable verbose output
python main.py animation.json skeleton.json animation.anim --verbose
```

> **Note:** Using `--fps 60` or `--no-fps-conversion` is recommended to avoid frame loss and preserve exact timing. The converter automatically removes duplicate frames during FPS conversion.

### Common FPS Values

- `--fps 24` - Film (Maya timeUnit: film)
- `--fps 29.97` - NTSC (Maya timeUnit: ntsc) **[default]**
- `--fps 30` - NTSC (Maya timeUnit: ntsc)
- `--fps 60` - NTSC Field (Maya timeUnit: ntscf)

## Importing into Maya

After generating the `.anim` file, import it into Maya:

### Using MEL Command

```mel
file -import -type "animImport" -ra true "path/to/animation.anim";
```

### Using Python in Maya

```python
import maya.cmds as cmds
cmds.file("path/to/animation.anim", i=True, type="animImport", ra=True)
```

### Using Maya UI

1. File → Import
2. Set "Files of type" to "animImport"
3. Select your `.anim` file
4. Click Import

## Project Structure

```
nuanmb_to_maya/
├── main.py                  # Main entry point
├── src/
│   ├── models.py           # Data structures
│   ├── math_utils.py       # Math utilities (quaternion conversion)
│   ├── nuanmb_parser.py    # JSON parser
│   ├── maya_writer.py      # Maya .anim writer
│   └── converter.py        # Main conversion logic
├── tests/                  # Unit tests
├── examples/               # Example files
└── README.md              # This file
```

## Technical Details

### Animation Data Flow

1. **Load Skeleton** - Load bone hierarchy from NUSKTB JSON file
2. **Topological Sort** - Sort bones by parent_index to ensure correct hierarchy (parents before children)
3. **Parse Animation** - Load NUANMB JSON exported by ssbh_data
4. **Extract Transform Groups** - Get bone animation data
5. **Sort Animation Bones** - Reorder animation bones to match hierarchical skeleton order
6. **Convert Quaternions** - Convert rotations to Euler angles (XYZ order)
7. **Adjust FPS** - Convert from 60fps to target Maya FPS
8. **Generate Curves** - Create Maya animation curves for each attribute
9. **Write File** - Output Maya-compatible ASCII .anim file

### Coordinate System

- NUANMB uses quaternions (x, y, z, w) for rotations
- Maya uses Euler angles (X, Y, Z) in degrees
- Conversion order: XYZ (Maya default)

### Supported Attributes

| NUANMB | Maya Attribute | Type |
|--------|---------------|------|
| translation.x/y/z | translate.translateX/Y/Z | Linear |
| rotation (quat) | rotate.rotateX/Y/Z | Angular |
| scale.x/y/z | scale.scaleX/Y/Z | Linear |

## Limitations

- Currently only supports Transform animations (bone animations)
- Visibility and Material animations are not yet supported
- Helper Bone constraints are not converted
- Scale compensation is not yet implemented
- Transform flags (override) are not yet applied

## Troubleshooting

### Import Error: "No object matches name"

Make sure your Maya scene has bones/joints with matching names to the animation data.

### Animation plays at wrong speed

Check the FPS setting. NUANMB animations are 60fps. Use `--fps` to match your Maya scene's time unit.

### Rotations look incorrect

The tool uses XYZ rotation order. If your Maya rig uses a different order, you may need to adjust it in Maya after import.

## Development

### Running Tests

```bash
python -m pytest tests/
```

### Code Style

All code follows PEP 8 guidelines with English comments and docstrings.

## Credits

- Based on [ssbh_wgpu](https://github.com/ScanMountGoat/ssbh_wgpu) animation implementation
- Uses [ssbh_data](https://docs.rs/ssbh_data/) for NUANMB parsing
- Part of the [SSBH Editor](https://github.com/ScanMountGoat/ssbh_editor) project

## License

This tool is part of the SSBH Editor project. See the main project for license information.

## Version History

### 0.2.0 (2025-10-05)
- Added skeleton bone ordering support with hierarchical topological sort
- Animation bones now sorted by parent_index hierarchy
- Ensures parent bones are always before children in output
- Fixes bone order mismatch when importing to Maya

### 0.1.0 (2025-10-04)
- Initial release
- Basic Transform animation conversion
- Quaternion to Euler conversion
- FPS conversion support
- Maya 2020+ compatibility

