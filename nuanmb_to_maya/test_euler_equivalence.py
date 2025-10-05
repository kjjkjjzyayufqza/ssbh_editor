"""
Test if (90, 90, 0) and (90, 0, -90) represent the same rotation
"""
import numpy as np
from src.models import Vector3, Vector4
from src.math_utils import quat_to_euler, build_matrix4x4

def euler_to_matrix(euler_x, euler_y, euler_z, order='XYZ'):
    """Convert Euler angles to rotation matrix"""
    # Convert to radians
    x = np.radians(euler_x)
    y = np.radians(euler_y)
    z = np.radians(euler_z)
    
    # Create rotation matrices
    Rx = np.array([
        [1, 0, 0],
        [0, np.cos(x), -np.sin(x)],
        [0, np.sin(x), np.cos(x)]
    ])
    
    Ry = np.array([
        [np.cos(y), 0, np.sin(y)],
        [0, 1, 0],
        [-np.sin(y), 0, np.cos(y)]
    ])
    
    Rz = np.array([
        [np.cos(z), -np.sin(z), 0],
        [np.sin(z), np.cos(z), 0],
        [0, 0, 1]
    ])
    
    # Combine in XYZ order
    if order == 'XYZ':
        R = Rz @ Ry @ Rx
    
    return R

def test_rotation_on_vector(rotation_matrix, vector):
    """Apply rotation to a vector"""
    return rotation_matrix @ vector

print("=" * 70)
print("Testing if (90, 90, 0) and (90, 0, -90) produce the same rotation")
print("=" * 70)

# Test with (90, 90, 0)
R1 = euler_to_matrix(90, 90, 0)
print("\nRotation matrix for Euler (90, 90, 0):")
print(R1)

# Test with (90, 0, -90)
R2 = euler_to_matrix(90, 0, -90)
print("\nRotation matrix for Euler (90, 0, -90):")
print(R2)

# Check if they're the same
print("\nAre the matrices identical?")
print(f"Max difference: {np.max(np.abs(R1 - R2)):.10f}")

if np.allclose(R1, R2, atol=1e-6):
    print("[YES] - These two Euler angle representations produce THE SAME rotation!")
    print("This is expected due to gimbal lock at 90 degree X rotation.")
else:
    print("[NO] - These produce DIFFERENT rotations!")
    print("==> This explains the user's bug report: animation plays on wrong axis!")

# Test on sample vectors
print("\n" + "=" * 70)
print("Testing rotation on sample vectors")
print("=" * 70)

test_vectors = [
    ("X-axis", np.array([1, 0, 0])),
    ("Y-axis", np.array([0, 1, 0])),
    ("Z-axis", np.array([0, 0, 1])),
    ("Diagonal", np.array([1, 1, 1]) / np.sqrt(3))
]

for name, vec in test_vectors:
    result1 = test_rotation_on_vector(R1, vec)
    result2 = test_rotation_on_vector(R2, vec)
    print(f"\n{name} [{vec[0]:.2f}, {vec[1]:.2f}, {vec[2]:.2f}]:")
    print(f"  After (90, 90, 0):   [{result1[0]:.6f}, {result1[1]:.6f}, {result1[2]:.6f}]")
    print(f"  After (90, 0, -90):  [{result2[0]:.6f}, {result2[1]:.6f}, {result2[2]:.6f}]")
    print(f"  Difference: {np.linalg.norm(result1 - result2):.10f}")
