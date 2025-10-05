"""
Test quaternion transformations to debug coordinate system conversion
"""
import numpy as np
import sys
sys.path.insert(0, 'src')

from src.models import Vector3, Vector4, Transform
from src.math_utils import quat_to_euler, axis_angle_to_quat, quat_multiply

# Test 1: Identity quaternion (no rotation)
print("=" * 60)
print("Test 1: Identity Quaternion (0, 0, 0, 1)")
print("=" * 60)

Q_identity = Vector4(x=0.0, y=0.0, z=0.0, w=1.0)
euler_identity = quat_to_euler(Q_identity, order='XYZ')
print(f"Identity -> Euler: ({euler_identity.x:.2f}, {euler_identity.y:.2f}, {euler_identity.z:.2f})")

# Test 2: X-axis 90 degree rotation
print("\n" + "=" * 60)
print("Test 2: X-axis 90° Rotation")
print("=" * 60)

Q_X_90 = axis_angle_to_quat(Vector3(x=1.0, y=0.0, z=0.0), 90.0)
print(f"Q_X_90: (x={Q_X_90.x:.4f}, y={Q_X_90.y:.4f}, z={Q_X_90.z:.4f}, w={Q_X_90.w:.4f})")
euler_x90 = quat_to_euler(Q_X_90, order='XYZ')
print(f"Q_X_90 -> Euler: ({euler_x90.x:.2f}, {euler_x90.y:.2f}, {euler_x90.z:.2f})")

# Test 3: Z-axis -90 degree rotation
print("\n" + "=" * 60)
print("Test 3: Z-axis -90° Rotation")
print("=" * 60)

Q_Z_N90 = axis_angle_to_quat(Vector3(x=0.0, y=0.0, z=1.0), -90.0)
print(f"Q_Z_-90: (x={Q_Z_N90.x:.4f}, y={Q_Z_N90.y:.4f}, z={Q_Z_N90.z:.4f}, w={Q_Z_N90.w:.4f})")
euler_zn90 = quat_to_euler(Q_Z_N90, order='XYZ')
print(f"Q_Z_-90 -> Euler: ({euler_zn90.x:.2f}, {euler_zn90.y:.2f}, {euler_zn90.z:.2f})")

# Test 4: Combined transformation Q_X_90 * Q_identity * Q_Z_-90
print("\n" + "=" * 60)
print("Test 4: Q_X_90 * Q_identity * Q_Z_-90")
print("=" * 60)

Q_temp = quat_multiply(Q_identity, Q_Z_N90)
print(f"Q_temp = Q_identity * Q_Z_-90: (x={Q_temp.x:.4f}, y={Q_temp.y:.4f}, z={Q_temp.z:.4f}, w={Q_temp.w:.4f})")
euler_temp = quat_to_euler(Q_temp, order='XYZ')
print(f"Q_temp -> Euler: ({euler_temp.x:.2f}, {euler_temp.y:.2f}, {euler_temp.z:.2f})")

Q_final = quat_multiply(Q_X_90, Q_temp)
print(f"Q_final = Q_X_90 * Q_temp: (x={Q_final.x:.4f}, y={Q_final.y:.4f}, z={Q_final.z:.4f}, w={Q_final.w:.4f})")
euler_final = quat_to_euler(Q_final, order='XYZ')
print(f"Q_final -> Euler: ({euler_final.x:.2f}, {euler_final.y:.2f}, {euler_final.z:.2f})")

# Test 5: What should the expected result be?
print("\n" + "=" * 60)
print("Test 5: Expected Result Analysis")
print("=" * 60)
print("Expected Euler angles from f00damagehi1_fixed.anim: (90, 0, -90)")
print(f"Actual result from conversion: ({euler_final.x:.2f}, {euler_final.y:.2f}, {euler_final.z:.2f})")
print("\nDifference analysis:")
print(f"  X: {euler_final.x - 90:.2f}°")
print(f"  Y: {euler_final.y - 0:.2f}°")
print(f"  Z: {euler_final.z - (-90):.2f}°")
