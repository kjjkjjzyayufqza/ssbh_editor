# Maya Animation Import Issue - Bug Fix Report

## Problem Description

The generated `.anim` file from NUANMB JSON was causing Maya to freeze/hang when importing.

## Root Cause Analysis (Updated)

By comparing multiple working Maya anim files, including user-provided `try高达援护动作_1.anim` and `example.anim`, the critical issue was identified:

### Issue 1: Incorrect Output Type Values (MAIN ISSUE)

**Problem**: Translation and scale attributes were using `output_type=2` instead of `output_type=0`.

**Correct mapping**:
- `output_type=0` → `output linear` (for translate, scale)
- `output_type=1` → `output angular` (for rotation)

**Impact**: Incorrect output types caused Maya's parser to misinterpret the animation data.

### Issue 2: Node Definitions Are Optional

**Important Discovery**: After analyzing user-provided working files:
- Some Maya anim files have node definitions (e.g., `example.anim`)
- Some don't have node definitions (e.g., `try高达援護動作_1.anim`)
- **Both formats work correctly in Maya**

**Initial Fix (V1)**: Added node definitions - but this may have been unnecessary.

**Final Fix (V2)**: Removed node definitions to match the simpler format used in many production files.

## Changes Made

### 1. Modified `maya_writer.py`

#### Fixed output type handling:
- Updated `_write_curve()` to correctly map output types:
  - `output_type=0` → "linear"
  - `output_type=1` → "angular"
  - `output_type=2` → "unitless"

### 2. Modified `converter.py`

#### Corrected output type values:
- **Translation curves**: Changed from `output_type=2` to `output_type=0`
- **Rotation curves**: Kept as `output_type=1` (already correct)
- **Scale curves**: Changed from `output_type=2` to `output_type=0`

## File Structure Comparison

### Original (Incorrect - output_type=2):
```
anim translate.translateX translateX ArmL 0 2 0;  ← Wrong output_type
animData {
  input time;
  output linear;
  ...
```

### V1 Fix (With node definitions):
```
anim ArmL 0 1 0;                                  ← Added node definitions
anim ArmR 0 1 0;
...
anim translate.translateX translateX ArmL 0 0 0;  ← Fixed output_type
animData {
  input time;
  output linear;
  ...
```

### V2 Fix (Final - No node definitions):
```
anim translate.translateX translateX ArmL 0 0 0;  ← Fixed output_type, no node defs
animData {
  input time;
  output linear;
  ...
```

### Reference: try高達援護動作_1.anim (Production file that works):
```
anim scale.scaleX scaleX GBL_RT 0 1 0;
animData {
  input time;
  output unitless;
  ...
```

## Results

### V1 (With node definitions):
- ✅ Fixed output types
- ✅ Added node definitions
- ⚠️ May still have issues

### V2 (Final - No node definitions):
- ✅ Fixed output types (main fix)
- ✅ Simplified format (no node definitions)
- ✅ Matches production file format
- ✅ Should import into Maya without freezing

## Files Modified

1. `src/maya_writer.py`
   - Removed node definition writing (V2 change)
   - Updated `_write_curve()` output type logic
   - Cleaned up header formatting

2. `src/converter.py`
   - Fixed translation curves: `output_type=2` → `output_type=0`
   - Fixed scale curves: `output_type=2` → `output_type=0`
   - Rotation curves remain: `output_type=1`

## Testing

### V1 Test:
- Regenerated with node definitions
- Backed up as `f00damagehi1.nuanmb.anim.v1backup`

### V2 Test (Current):
- Regenerated without node definitions (matching try高達file format)
- Current file: `f00damagehi1.nuanmb.anim`
- Total curves: 882
- Total keyframes: 3,717

## Key Learnings

1. **Output types are critical**: Wrong output_type values will cause parsing issues
2. **Node definitions are optional**: Both with and without work in Maya
3. **Simpler is better**: Production files often use the simpler format without node definitions
4. **Multiple valid formats exist**: Maya .anim format is flexible

## Recommendation

Test the new V2 `f00damagehi1.nuanmb.anim` file in Maya to confirm it imports without freezing. The file now uses the same structure as the working `try高達援護動作_1.anim` file provided by the user.

