"""
Test which Euler representation is correct for the transformed matrix
"""
import numpy as np

def euler_to_rotation_matrix(euler_x, euler_y, euler_z):
    """Convert Euler XYZ to rotation matrix"""
    x, y, z = np.radians([euler_x, euler_y, euler_z])
    
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
    
    # XYZ order: apply Z, then Y, then X
    return Rz @ Ry @ Rx

print("=" * 70)
print("Which Euler representation matches the corrected matrix?")
print("=" * 70)

# The corrected matrix from previous test
M_corr_rotation = np.array([
    [ 0.,  1.,  0.],
    [ 0.,  0., -1.],
    [-1.,  0.,  0.]
])

print("\nTarget Matrix (M_corr rotation part):")
print(M_corr_rotation)

# Test (90, 90, 0)
R1 = euler_to_rotation_matrix(90, 90, 0)
print("\nMatrix from Euler (90, 90, 0):")
print(R1)
print(f"Difference from target: {np.max(np.abs(R1 - M_corr_rotation)):.6f}")

# Test (90, 0, -90)
R2 = euler_to_rotation_matrix(90, 0, -90)
print("\nMatrix from Euler (90, 0, -90):")
print(R2)
print(f"Difference from target: {np.max(np.abs(R2 - M_corr_rotation)):.6f}")

print("\n" + "=" * 70)
print("Conclusion:")
print("=" * 70)

if np.allclose(R1, M_corr_rotation, atol=1e-6):
    print("(90, 90, 0) is CORRECT for this transformation!")
    print("The current implementation is producing the RIGHT Euler angles.")
    print("==> The bug might be elsewhere, or the 'expected' values are wrong.")
elif np.allclose(R2, M_corr_rotation, atol=1e-6):
    print("(90, 0, -90) is CORRECT for this transformation!")
    print("We need to fix the quat_to_euler function to produce this representation.")
else:
    print("NEITHER matches! There's a deeper problem with the transformation.")
