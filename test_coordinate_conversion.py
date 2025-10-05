"""
Test script to verify coordinate system conversion correctness
"""
import numpy as np
import sys
import os
sys.path.append('nuanmb_to_maya/src')

from nuanmb_to_maya.src.math_utils import build_matrix4x4, axis_angle_to_quat, quat_multiply, Vector3, Vector4

def test_coordinate_conversion():
    # Original SSBH transform: CENTER_RT (x=0, y=10, z=0)
    T = Vector3(x=0.0, y=10.0, z=0.0)
    R = Vector4(x=0.0, y=0.0, z=0.0, w=1.0)  # Identity rotation
    S = Vector3(x=1.0, y=1.0, z=1.0)

    # Build SSBH matrix
    M_ssbh = build_matrix4x4(T, R, S)
    print('SSBH matrix:')
    print(M_ssbh)
    print()

    # Define transformation matrices (from the code)
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

    print('R_X_90 matrix:')
    print(R_X_90)
    print()

    print('R_Z_N90 matrix:')
    print(R_Z_N90)
    print()

    # Apply full transformation (as used for root bone)
    M_corr = R_X_90 @ M_ssbh @ R_Z_N90
    T_corr = Vector3(x=M_corr[0, 3], y=M_corr[1, 3], z=M_corr[2, 3])

    print('Full transformation result:')
    print(f'T_corr: ({T_corr.x}, {T_corr.y}, {T_corr.z})')
    print()

    # Test corrected simplified mapping (after fix)
    print('Corrected simplified mapping result:')
    x_new = T.x      # 0
    y_new = T.z      # 0
    z_new = T.y      # 10 (FIXED: was -T.y = -10)
    print(f'T_simple_fixed: ({x_new}, {y_new}, {z_new})')
    print()

    # Check if they match
    print('Do they match?')
    print(f'X: {abs(T_corr.x - x_new) < 1e-6}')
    print(f'Y: {abs(T_corr.y - y_new) < 1e-6}')
    print(f'Z: {abs(T_corr.z - z_new) < 1e-6}')
    print()

    # Let's also test what R_X_90 alone does to the translation
    print('R_X_90 applied to translation vector directly:')
    trans_vector = np.array([T.x, T.y, T.z])
    R_X_90_only = R_X_90[:3, :3]  # Just the rotation part
    trans_x90 = R_X_90_only @ trans_vector
    print(f'R_X_90 @ [0,10,0] = [{trans_x90[0]}, {trans_x90[1]}, {trans_x90[2]}]')
    print()

    # Let's also test what R_Z_N90 does to the result
    print('R_Z_N90 applied to R_X_90 result:')
    R_Z_N90_only = R_Z_N90[:3, :3]  # Just the rotation part
    trans_final = R_Z_N90_only @ trans_x90
    print(f'R_Z_N90 @ [0,0,10] = [{trans_final[0]}, {trans_final[1]}, {trans_final[2]}]')
    print()

    # Let's check the matrix multiplication order
    print('Checking matrix multiplication order:')
    print('Order in smash-ultimate-blender: y_up_to_z_up @ transform @ x_major_to_y_major')
    print('Which is: R_X_90 @ M_ssbh @ R_Z_N90')
    print()

    # Let's see what happens if we apply transforms differently
    print('Alternative: What if we apply R_X_90 only (like the simplified mapping should do)?')
    # The simplified mapping does: X=X, Y=Z, Z=-Y
    # This is equivalent to applying R_X_90 to the translation vector directly
    print(f'R_X_90 @ [0,10,0] = [0, 0, 10]')
    print('But simplified mapping gives: [0, 0, -10]')
    print('So the simplified mapping is WRONG - it should be [0, 0, 10]')

    # Let's check what the correct mapping should be
    print()
    print('Correct mapping for non-root bones should be:')
    print('X_new = X_old')
    print('Y_new = Z_old')
    print('Z_new = Y_old')  # NOT -Y_old
    print(f'So for [0,10,0] -> [{T.x}, {T.z}, {T.y}] = [0, 0, 10]')

if __name__ == '__main__':
    test_coordinate_conversion()
