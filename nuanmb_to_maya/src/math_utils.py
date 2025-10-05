"""
Mathematical utilities for animation data conversion.
Includes quaternion to Euler angle conversion and interpolation functions.
"""

import numpy as np
from typing import Tuple
from .models import Vector3, Vector4, Transform


def quat_to_euler(q: Vector4, order: str = 'XYZ') -> Vector3:
    """
    Convert quaternion to Euler angles (in degrees).
    
    Args:
        q: Quaternion (x, y, z, w)
        order: Rotation order ('XYZ', 'YZX', 'ZXY', etc.)
    
    Returns:
        Euler angles in degrees
    """
    # Normalize quaternion
    x, y, z, w = q.x, q.y, q.z, q.w
    norm = np.sqrt(x*x + y*y + z*z + w*w)
    
    # Handle zero quaternion
    if norm < 1e-10:
        return Vector3(x=0.0, y=0.0, z=0.0)
    
    x, y, z, w = x/norm, y/norm, z/norm, w/norm
    
    # Convert to rotation matrix
    rot_mat = np.array([
        [1 - 2*(y*y + z*z), 2*(x*y - w*z), 2*(x*z + w*y)],
        [2*(x*y + w*z), 1 - 2*(x*x + z*z), 2*(y*z - w*x)],
        [2*(x*z - w*y), 2*(y*z + w*x), 1 - 2*(x*x + y*y)]
    ])
    
    # Extract Euler angles based on order
    if order == 'XYZ':
        # Extract XYZ Euler angles
        sy = np.sqrt(rot_mat[0, 0]**2 + rot_mat[1, 0]**2)
        
        singular = sy < 1e-6
        
        if not singular:
            euler_x = np.arctan2(rot_mat[2, 1], rot_mat[2, 2])
            euler_y = np.arctan2(-rot_mat[2, 0], sy)
            euler_z = np.arctan2(rot_mat[1, 0], rot_mat[0, 0])
        else:
            euler_x = np.arctan2(-rot_mat[1, 2], rot_mat[1, 1])
            euler_y = np.arctan2(-rot_mat[2, 0], sy)
            euler_z = 0
        
        return Vector3(
            x=np.degrees(euler_x),
            y=np.degrees(euler_y),
            z=np.degrees(euler_z)
        )
    else:
        raise NotImplementedError(f"Rotation order {order} not implemented")


def build_matrix4x4(t: Vector3, r: Vector4, s: Vector3) -> np.ndarray:
    """Builds a 4x4 matrix from translation, rotation (quaternion), and scale (T*R*S)."""
    # SSBH quaternions are X, Y, Z, W storage order
    x, y, z, w = r.x, r.y, r.z, r.w
    
    # Calculate rotation matrix components from quaternion
    xx, yy, zz = x * x, y * y, z * z
    xy, xz, yz = x * y, x * z, y * z
    xw, yw, zw = x * w, y * w, z * w

    rot_mat = np.array([
        [1 - 2 * (yy + zz),     2 * (xy - zw),     2 * (xz + yw)],
        [    2 * (xy + zw), 1 - 2 * (xx + zz),     2 * (yz - xw)],
        [    2 * (xz - yw),     2 * (yz + xw), 1 - 2 * (xx + yy)]
    ])

    # Apply scale to rotation matrix
    scaled_rot_mat = rot_mat * [s.x, s.y, s.z]

    mat = np.identity(4)
    mat[:3, :3] = scaled_rot_mat
    mat[0, 3] = t.x
    mat[1, 3] = t.y
    mat[2, 3] = t.z
    
    # Note: SSBH uses column-major order for translation in its internal representation, but its matrix multiplication order T*R*S generally assumes translation is in column 3.
    # Given the Blender reference used T*R*S order, we build it the standard way (translation in the fourth column).
    return mat


def quat_from_matrix(matrix: np.ndarray) -> Vector4:
    """Extract quaternion (x, y, z, w) from the 4x4 rotation matrix part."""
    # Ensure rotation matrix is extracted (3x3 part) and normalized for potential scale issues
    M = matrix[:3, :3]
    # Handle scale: Q is only defined for orthogonal matrices (no scale).
    # Since the input matrix handles T*R*S, we only need the R part.
    # To get R from R*S, we need to divide by scale, but robustly decomposing a general 3x3 matrix into Q is complex.
    # Assume rotation part of M is orthogonal IF input comes from a matrix generated without scale.
    # However, if we decompose T*R*S, we must first remove the scale.
    
    scale_x = np.linalg.norm(M[0, :])
    scale_y = np.linalg.norm(M[1, :])
    scale_z = np.linalg.norm(M[2, :])
    
    # If matrix columns are not normalized, normalize them to extract rotation
    if abs(scale_x - 1.0) > 1e-6 or abs(scale_y - 1.0) > 1e-6 or abs(scale_z - 1.0) > 1e-6:
        # Use pseudo-polar decomposition or just extract rotation part assuming orthogonal decomposition
        # Since we are trying to perfectly emulate what Blender did when it decomposed its resulting matrix (M_corrected),
        # we will use a common simplified approach assuming M is M_rot * M_scale
        
        # Normalize rows to approximate the rotation matrix (ignoring shear)
        R = np.copy(M)
        if scale_x != 0: R[0, :] /= scale_x
        if scale_y != 0: R[1, :] /= scale_y
        if scale_z != 0: R[2, :] /= scale_z
    else:
        R = M

    # Standard matrix to quaternion conversion (SSBH order: X, Y, Z, W)
    
    # Trace method
    tr = R[0, 0] + R[1, 1] + R[2, 2]
    
    if tr > 0:
        S = np.sqrt(tr + 1.0) * 2
        w = 0.25 * S
        x = (R[2, 1] - R[1, 2]) / S
        y = (R[0, 2] - R[2, 0]) / S
        z = (R[1, 0] - R[0, 1]) / S
    elif (R[0, 0] > R[1, 1]) and (R[0, 0] > R[2, 2]):
        S = np.sqrt(1.0 + R[0, 0] - R[1, 1] - R[2, 2]) * 2
        w = (R[2, 1] - R[1, 2]) / S
        x = 0.25 * S
        y = (R[0, 1] + R[1, 0]) / S
        z = (R[0, 2] + R[2, 0]) / S
    elif R[1, 1] > R[2, 2]:
        S = np.sqrt(1.0 + R[1, 1] - R[0, 0] - R[2, 2]) * 2
        w = (R[0, 2] - R[2, 0]) / S
        x = (R[0, 1] + R[1, 0]) / S
        y = 0.25 * S
        z = (R[1, 2] + R[2, 1]) / S
    else:
        S = np.sqrt(1.0 + R[2, 2] - R[0, 0] - R[1, 1]) * 2
        w = (R[1, 0] - R[0, 1]) / S
        x = (R[0, 2] + R[2, 0]) / S
        y = (R[1, 2] + R[2, 1]) / S
        z = 0.25 * S
    
    return Vector4(x=x, y=y, z=z, w=w)


def matrix_to_trans_quat_scale(matrix: np.ndarray) -> Tuple[Vector3, Vector4, Vector3]:
    """Decompose a 4x4 matrix into translation, rotation (quaternion), and scale."""
    
    t = Vector3(x=matrix[0, 3], y=matrix[1, 3], z=matrix[2, 3])
    
    # Extract scale by column normalization
    scale_x = np.linalg.norm(matrix[0, :3])
    scale_y = np.linalg.norm(matrix[1, :3])
    scale_z = np.linalg.norm(matrix[2, :3])
    s = Vector3(x=scale_x, y=scale_y, z=scale_z)

    # Extract rotation part by normalizing the 3x3 rotational sub-matrix
    R = np.copy(matrix[:3, :3])
    if scale_x != 0: R[0, :] /= scale_x
    if scale_y != 0: R[1, :] /= scale_y
    if scale_z != 0: R[2, :] /= scale_z

    q = quat_from_matrix(R)
    
    return t, q, s


def lerp_vector3(a: Vector3, b: Vector3, t: float) -> Vector3:
    """
    Linear interpolation for Vector3.
    ... [content remains the same] ...
    """
    return Vector3(
        x=a.x * (1 - t) + b.x * t,
        y=a.y * (1 - t) + b.y * t,
        z=a.z * (1 - t) + b.z * t
    )


def quat_multiply(q1: Vector4, q2: Vector4) -> Vector4:
    """
    Multiply two quaternions: result = q1 * q2
    
    Args:
        q1: First quaternion (x, y, z, w)
        q2: Second quaternion (x, y, z, w)
    
    Returns:
        Product quaternion
    """
    w1, x1, y1, z1 = q1.w, q1.x, q1.y, q1.z
    w2, x2, y2, z2 = q2.w, q2.x, q2.y, q2.z
    
    w = w1*w2 - x1*x2 - y1*y2 - z1*z2
    x = w1*x2 + x1*w2 + y1*z2 - z1*y2
    y = w1*y2 - x1*z2 + y1*w2 + z1*x2
    z = w1*z2 + x1*y2 - y1*x2 + z1*w2
    
    return Vector4(x=x, y=y, z=z, w=w)


def axis_angle_to_quat(axis: Vector3, angle_deg: float) -> Vector4:
    """
    Create a quaternion from axis-angle representation.
    
    Args:
        axis: Rotation axis (should be normalized)
        angle_deg: Rotation angle in degrees
    
    Returns:
        Quaternion representing the rotation
    """
    angle_rad = np.radians(angle_deg)
    half_angle = angle_rad / 2.0
    s = np.sin(half_angle)
    c = np.cos(half_angle)
    
    return Vector4(
        x=axis.x * s,
        y=axis.y * s,
        z=axis.z * s,
        w=c
    )


def slerp_quat(a: Vector4, b: Vector4, t: float) -> Vector4:
    """
    Spherical linear interpolation for quaternions.
    ... [content remains the same] ...
    """
    # Normalize quaternions
    a_arr = np.array([a.x, a.y, a.z, a.w])
    b_arr = np.array([b.x, b.y, b.z, b.w])
    
    a_norm = np.linalg.norm(a_arr)
    b_norm = np.linalg.norm(b_arr)
    
    if a_norm < 1e-10 or b_norm < 1e-10:
        # Return identity quaternion if either is zero
        return Vector4(x=0.0, y=0.0, z=0.0, w=1.0)
    
    a_arr = a_arr / a_norm
    b_arr = b_arr / b_norm
    
    # Calculate dot product
    dot = np.dot(a_arr, b_arr)
    
    # If dot is negative, negate one quaternion to take shorter path
    if dot < 0.0:
        b_arr = -b_arr
        dot = -dot
    
    # Clamp dot to avoid numerical errors
    dot = np.clip(dot, -1.0, 1.0)
    
    # Perform slerp
    theta = np.arccos(dot)
    sin_theta = np.sin(theta)
    
    if sin_theta < 1e-6:
        # Linear interpolation for very close quaternions
        result = a_arr * (1 - t) + b_arr * t
    else:
        result = (np.sin((1 - t) * theta) / sin_theta) * a_arr + \
                 (np.sin(t * theta) / sin_theta) * b_arr
    
    return Vector4(x=result[0], y=result[1], z=result[2], w=result[3])


def interpolate_transform(a: Transform, b: Transform, t: float) -> Transform:
    """
    Interpolate between two transforms.
    ... [content remains the same] ...
    """
    return Transform(
        translation=lerp_vector3(a.translation, b.translation, t),
        rotation=slerp_quat(a.rotation, b.rotation, t),
        scale=lerp_vector3(a.scale, b.scale, t)
    )
