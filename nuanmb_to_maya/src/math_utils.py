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


def lerp_vector3(a: Vector3, b: Vector3, t: float) -> Vector3:
    """
    Linear interpolation for Vector3.
    
    Args:
        a: Start vector
        b: End vector
        t: Interpolation factor (0.0 to 1.0)
    
    Returns:
        Interpolated vector
    """
    return Vector3(
        x=a.x * (1 - t) + b.x * t,
        y=a.y * (1 - t) + b.y * t,
        z=a.z * (1 - t) + b.z * t
    )


def slerp_quat(a: Vector4, b: Vector4, t: float) -> Vector4:
    """
    Spherical linear interpolation for quaternions.
    
    Args:
        a: Start quaternion
        b: End quaternion
        t: Interpolation factor (0.0 to 1.0)
    
    Returns:
        Interpolated quaternion
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
    
    Args:
        a: Start transform
        b: End transform
        t: Interpolation factor (0.0 to 1.0)
    
    Returns:
        Interpolated transform
    """
    return Transform(
        translation=lerp_vector3(a.translation, b.translation, t),
        rotation=slerp_quat(a.rotation, b.rotation, t),
        scale=lerp_vector3(a.scale, b.scale, t)
    )

