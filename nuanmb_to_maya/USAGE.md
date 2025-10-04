# NUANMB to Maya Converter - Usage Guide

## Quick Start

### Basic Conversion

```bash
# 1. Convert NUANMB to JSON using ssbh_data
ssbh_data_json animation.nuanmb animation.json

# 2. Convert JSON to Maya .anim (default: 29.97fps)
python main.py animation.json animation.anim
```

## Common Use Cases

### Keep Original 60fps (Recommended)

To avoid frame loss and maintain timing accuracy:

```bash
python main.py animation.json animation.anim --fps 60
# OR
python main.py animation.json animation.anim --no-fps-conversion
```

**Why use 60fps?**
- ✅ No frame duplication or loss
- ✅ Preserves exact timing
- ✅ Faster import into Maya
- ✅ Smaller file size
- ⚠️ May need to set Maya timeline to 60fps

### Convert to Standard Maya FPS

```bash
# Film (24fps)
python main.py animation.json animation.anim --fps 24

# NTSC (29.97fps) - default
python main.py animation.json animation.anim --fps 29.97

# NTSC (30fps)
python main.py animation.json animation.anim --fps 30
```

## Performance Comparison

### Example: 12-frame animation with 98 bones

| FPS Setting | Keyframes | File Size | Import Speed |
|-------------|-----------|-----------|--------------|
| 60fps (no conversion) | ~8,800 | ~12,000 lines | ⚡ Fast |
| 29.97fps (default) | ~3,700 | ~11,000 lines | ✓ Normal |
| 24fps | ~3,000 | ~9,000 lines | ✓ Normal |

## Command Line Options

```bash
python main.py INPUT OUTPUT [OPTIONS]
```

### Required Arguments

- `INPUT` - Input JSON file (from ssbh_data)
- `OUTPUT` - Output Maya .anim file

### Optional Arguments

- `--fps FLOAT` - Target Maya FPS (default: 29.97)
  - Common values: 24, 29.97, 30, 60
  
- `--no-fps-conversion` - Keep original 60fps (recommended)

- `--maya-version STRING` - Maya version (default: 2020)

- `--verbose` - Show detailed output and error traces

## Examples

### Example 1: Fighter Animation (Recommended)

```bash
# Keep original timing for precise playback
python main.py mario_attack.json mario_attack.anim --fps 60
```

### Example 2: Cinematic Export

```bash
# Convert to film rate for video editing
python main.py cutscene.json cutscene.anim --fps 24
```

### Example 3: Batch Processing

```bash
# Windows
for %f in (*.json) do python main.py "%f" "%~nf.anim" --fps 60

# Linux/Mac
for f in *.json; do python main.py "$f" "${f%.json}.anim" --fps 60; done
```

## Troubleshooting

### Maya Freezes During Import

**Cause:** Old version with duplicate keyframes
**Solution:** Re-generate with updated converter (duplicate frames now removed automatically)

```bash
python main.py animation.json animation_fixed.anim --fps 60
```

### Animation Plays at Wrong Speed

**Cause:** FPS mismatch between .anim file and Maya timeline

**Solution:**
1. Check your Maya timeline FPS settings
2. If using `--fps 60`, set Maya to 60fps:
   - Window → Settings/Preferences → Preferences → Settings → Time → FPS: 60 fps

### Missing Keyframes

**Cause:** FPS conversion caused frame merging

**Solution:** Use `--no-fps-conversion` to preserve all frames:

```bash
python main.py animation.json animation.anim --no-fps-conversion
```

### File Too Large

**Cause:** Many bones or long animation

**Solution:** This is normal. Maya can handle large .anim files. If needed:
- Split animation into shorter clips
- Remove unused bones from source

## Maya Import Instructions

### Method 1: MEL Command

```mel
file -import -type "animImport" -ra true "C:/path/to/animation.anim";
```

### Method 2: Python Command

```python
import maya.cmds as cmds
cmds.file("C:/path/to/animation.anim", i=True, type="animImport", ra=True)
```

### Method 3: UI

1. File → Import
2. Set "Files of type" to "animImport"
3. Select your `.anim` file
4. Click Import

## Best Practices

### 1. Match Bone Names

Ensure your Maya rig has bones with the same names as in the NUANMB file.

### 2. Set FPS First

Set Maya's FPS before importing to avoid timing issues.

### 3. Use 60fps for Game Dev

For game development workflows, keep 60fps to match in-game playback.

### 4. Use 24fps for Video

For video editing or cinematics, convert to 24fps.

### 5. Batch Convert

Process multiple files at once to save time:

```bash
# Windows PowerShell
Get-ChildItem *.json | ForEach-Object { 
    python main.py $_.Name "$($_.BaseName).anim" --fps 60 
}
```

## Technical Notes

### FPS Conversion Behavior

- **60fps → 29.97fps**: Every ~2 frames merged into 1
- **60fps → 24fps**: Every 2.5 frames merged into 1
- **60fps → 60fps**: No conversion (1:1 mapping)

### Duplicate Frame Handling

The converter automatically removes duplicate frame numbers that occur during FPS conversion, keeping the last value for each Maya frame.

### Rotation Order

All rotations are converted to XYZ Euler order (Maya default). If your rig uses a different order, you may need to adjust in Maya after import.

## Support

For issues or questions:
- Check the main README.md
- Review Python转换工具开发需求.md for technical details
- Report bugs with the `--verbose` flag output

