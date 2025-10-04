# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2025-10-04

### Fixed
- **Critical: Removed duplicate keyframes** that caused Maya to freeze during import
  - Problem: FPS conversion (60fps → 29.97fps) created multiple keyframes with the same frame number
  - Solution: Added duplicate frame detection and removal in all keyframe creation functions
  - Result: File size reduced by ~28%, much faster Maya import

### Added
- `--no-fps-conversion` flag to keep original 60fps framerate
- Detailed `USAGE.md` guide with examples and troubleshooting
- Automatic duplicate frame removal during FPS conversion

### Changed
- Updated README.md to recommend using `--fps 60` for best results
- Improved conversion output messages with keyframe count statistics

### Technical Details

#### Before Fix (v0.1.0)
```
Example: 12-frame animation, 98 bones
- Output: 15,064 lines
- Keyframes: ~10,584 (with duplicates)
- Maya import: Freezes ❌
```

#### After Fix (v0.1.1)
```
29.97fps conversion:
- Output: 10,778 lines (-28%)
- Keyframes: 3,717 (unique only)
- Maya import: Fast ✅

60fps (no conversion):
- Output: ~15,000 lines
- Keyframes: 7,119 (all original frames)
- Maya import: Very fast ✅
```

## [0.1.0] - 2025-10-04

### Added
- Initial release
- Convert NUANMB JSON to Maya .anim format
- Quaternion to Euler angle conversion (XYZ order)
- FPS conversion support (60fps to 24/29.97/30/60fps)
- Support for Translation, Rotation, and Scale animations
- Command-line interface with options for FPS and Maya version
- Complete documentation (README.md, requirements.txt, setup.py)

### Known Issues (Fixed in 0.1.1)
- Duplicate keyframes when converting FPS (fixed)
- Maya freeze on import large files (fixed)

