# Coordinate System Analysis

## Problem Statement
User reports: "动画变成在-Y轴上播放了，而不是smash-ultimate-blender-main的那样在正确的Y轴上apply"

Translation: "Animation is playing along the -Y axis, instead of applying correctly on the Y axis like smash-ultimate-blender-main"

## Key Findings from Tests

### 1. Root Bone Rotation
- Current implementation produces: `(90, 90, 0)`
- Previously thought "correct": `(90, 0, -90)`
- **Test Result**: `(90, 90, 0)` IS mathematically correct for the transformation matrix!

### 2. Coordinate Mapping Currently Applied (Non-Root Bones)
```python
X_new = X_raw      # X stays X
Y_new = Z_raw      # Z becomes Y  
Z_new = -Y_raw     # -Y becomes Z
```

This means:
- Movement along SSBH's Z axis → Maya's Y axis
- Movement along SSBH's Y axis → Maya's -Z axis
- Movement along SSBH's X axis → Maya's X axis

### 3. Possible Root Causes

#### Hypothesis A: Coordinate mapping is backwards
If animation "plays on -Y instead of Y", maybe:
- Original animation moves in +Z (SSBH's "up")
- Should map to +Y (Maya's "up")
- But currently maps to something else?

#### Hypothesis B: Root bone shouldn't be transformed
Maybe the root bone transformation is unnecessary and causing issues?

#### Hypothesis C: ALL bones need the same treatment
Maybe distinguishing root vs non-root is wrong?

## Next Steps

Need to test with actual animation data that has movement to see which axis the movement appears on.

## Smash Ultimate Blender Reference

From `import_model.py`:
```python
def get_blender_transform(m) -> Matrix:
    # In Ultimate, the bone's x-axis points from parent to child.
    # In Blender, the bone's y-axis points from parent to child.
    p = Matrix([
        [0, -1, 0, 0],
        [1, 0, 0, 0],
        [0, 0, 1, 0],
        [0, 0, 0, 1]
    ])
    return p @ m @ p.inverted()
```

This is for **bone-local** coordinate system (X-major to Y-major), NOT world coordinates!

From `import_anim.py` for root bone:
```python
y_up_to_z_up = Matrix.Rotation(math.radians(90), 4, 'X')
x_major_to_y_major = Matrix.Rotation(math.radians(-90), 4, 'Z')
bone.matrix = y_up_to_z_up @ raw_matrix @ x_major_to_y_major
```

This is for **world** coordinate system transformation.

## Conclusion

The issue is complex because there are TWO different coordinate transformations:
1. **World coordinates**: Z-up (SSBH) ↔ Y-up (Blender/Maya)
2. **Bone-local coordinates**: X-major (SSBH) ↔ Y-major (Blender)

Current implementation may be mixing these two concepts!
