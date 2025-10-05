"""
Test matrix decomposition approach for root bone transformation
"""
import numpy as np
from src.models import Vector3, Vector4, Transform
from src.math_utils import build_matrix4x4, matrix_to_trans_quat_scale, quat_to_euler

print("=" * 70)
print("Testing Matrix-based Root Bone Transformation")
print("=" * 70)

# Start with identity transform (no rotation)
T = Vector3(x=0.0, y=0.0, z=0.0)
R = Vector4(x=0.0, y=0.0, z=0.0, w=1.0)
S = Vector3(x=1.0, y=1.0, z=1.0)

print("\nOriginal Transform:")
print(f"  Translation: ({T.x}, {T.y}, {T.z})")
print(f"  Rotation (quat): ({R.x}, {R.y}, {R.z}, {R.w})")
print(f"  Scale: ({S.x}, {S.y}, {S.z})")

# Build SSBH matrix
M_ssbh = build_matrix4x4(T, R, S)
print("\nSSBH Matrix M:")
print(M_ssbh)

# Define correction matrices
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

print("\nR_X_90 (Rotate 90° around X):")
print(R_X_90)

print("\nR_Z_-90 (Rotate -90° around Z):")
print(R_Z_N90)

# Apply transformation
M_corr = R_X_90 @ M_ssbh @ R_Z_N90
print("\nCorrected Matrix M_corr = R_X_90 @ M @ R_Z_-90:")
print(M_corr)

# Decompose
T_new, Q_new, S_new = matrix_to_trans_quat_scale(M_corr)
print("\nDecomposed Transform:")
print(f"  Translation: ({T_new.x}, {T_new.y}, {T_new.z})")
print(f"  Rotation (quat): ({Q_new.x:.4f}, {Q_new.y:.4f}, {Q_new.z:.4f}, {Q_new.w:.4f})")
print(f"  Scale: ({S_new.x}, {S_new.y}, {S_new.z})")

# Convert to Euler
euler = quat_to_euler(Q_new, order='XYZ')
print(f"  Rotation (Euler XYZ): ({euler.x:.2f}, {euler.y:.2f}, {euler.z:.2f})")

print("\n" + "=" * 70)
print("Analysis:")
print("=" * 70)
print(f"Expected Euler: (90, 0, -90)")
print(f"Actual Euler:   ({euler.x:.2f}, {euler.y:.2f}, {euler.z:.2f})")

if abs(euler.x - 90) < 0.1 and abs(euler.y - 0) < 0.1 and abs(euler.z - (-90)) < 0.1:
    print("\n[SUCCESS] Euler angles match expected values!")
else:
    print("\n[PROBLEM] Euler angles don't match!")
    print("This is the source of the coordinate system bug.")
