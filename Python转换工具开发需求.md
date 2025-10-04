# NUANMB to Maya Anim 转换工具开发需求文档

## 项目概述

本文档详细说明了如何开发一个Python工具，将《任天堂明星大乱斗特别版》的 `.nuanmb` 动画文件转换为Maya可用的 `.anim` 文件。

### 工作流程

```
.nuanmb → [ssbh_data] → .json → [Python Tool] → .anim (Maya)
```

---

## 目录

1. [技术栈](#技术栈)
2. [NUANMB JSON格式分析](#nuanmb-json格式分析)
3. [ssbh_wgpu Apply_Anims 实现分析](#ssbh_wgpu-apply_anims-实现分析)
4. [Maya Anim文件格式](#maya-anim文件格式)
5. [Python转换逻辑设计](#python转换逻辑设计)
6. [核心算法实现](#核心算法实现)
7. [开发路线图](#开发路线图)

---

## 技术栈

### 依赖工具
- **ssbh_data** - Rust库，用于将nuanmb转换为JSON
- **Python 3.8+** - 转换脚本开发语言
- **Maya 2020+** - 目标3D软件

### Python库依赖
```python
json           # JSON解析
numpy          # 数学计算（四元数、矩阵）
dataclasses    # 数据类定义
typing         # 类型注解
argparse       # 命令行参数
```

---

## NUANMB JSON格式分析

### 1. 使用ssbh_data转换

首先需要使用ssbh_data工具将nuanmb转换为JSON：

```bash
# 假设使用 ssbh_data_json 工具
ssbh_data_json animation.nuanmb animation.json
```

### 2. JSON数据结构

基于 `ssbh_data` 库的 `AnimData` 结构，JSON格式如下：

```json
{
  "major_version": 2,
  "minor_version": 0,
  "final_frame_index": 120.0,
  "groups": [
    {
      "group_type": "Transform",
      "nodes": [
        {
          "name": "BoneName",
          "tracks": [
            {
              "name": "Transform",
              "compensate_scale": false,
              "transform_flags": {
                "override_translation": false,
                "override_rotation": false,
                "override_scale": false,
                "override_compensate_scale": false
              },
              "values": {
                "Transform": [
                  {
                    "translation": {"x": 0.0, "y": 0.0, "z": 0.0},
                    "rotation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0},
                    "scale": {"x": 1.0, "y": 1.0, "z": 1.0}
                  }
                ]
              }
            }
          ]
        }
      ]
    },
    {
      "group_type": "Visibility",
      "nodes": [...]
    },
    {
      "group_type": "Material",
      "nodes": [...]
    }
  ]
}
```

### 3. 关键数据类型

#### GroupType枚举
- `Transform` - 骨骼变换动画
- `Visibility` - 可见性动画（Boolean）
- `Material` - 材质动画

#### TrackValues类型
```json
// Transform类型 - 用于骨骼
{
  "Transform": [
    {
      "translation": {"x": float, "y": float, "z": float},
      "rotation": {"x": float, "y": float, "z": float, "w": float},  // 四元数
      "scale": {"x": float, "y": float, "z": float}
    }
  ]
}

// Float类型 - 用于材质参数
{
  "Float": [1.0, 0.5, 0.3, ...]
}

// Boolean类型 - 用于可见性
{
  "Boolean": [true, false, true, ...]
}

// Vector4类型 - 用于颜色等
{
  "Vector4": [
    {"x": 1.0, "y": 0.0, "z": 0.0, "w": 1.0}
  ]
}

// UvTransform类型 - 用于UV动画
{
  "UvTransform": [
    {
      "scale_u": 1.0,
      "scale_v": 1.0,
      "rotation": 0.0,
      "translate_u": 0.0,
      "translate_v": 0.0
    }
  ]
}
```

---

## ssbh_wgpu Apply_Anims 实现分析

### 核心文件位置
- `E:\research\ssbh_wgpu\ssbh_wgpu\src\animation.rs`
- `E:\research\ssbh_wgpu\ssbh_wgpu\src\model.rs`

### 1. 动画应用主流程

**文件**: `model.rs:205-263`

```rust
pub fn apply_anims(
    &mut self,
    queue: &wgpu::Queue,
    anims: impl Iterator<Item = &AnimData>,
    skel: Option<&SkelData>,
    matl: Option<&MatlData>,
    hlpb: Option<&HlpbData>,
    shared_data: &SharedRenderData,
    current_frame: f32,
)
```

#### 处理步骤：

1. **可见性动画处理**
```rust
animate_visibility(anim, current_frame, &mut self.meshes);
```

2. **材质动画处理**
```rust
self.update_material_uniforms(anim, current_frame, matl, shared_data, queue);
```

3. **骨骼动画处理**（核心）
```rust
animate_skel(
    &mut self.animation_transforms,
    skel,
    anims,
    hlpb,
    current_frame,
);
```

### 2. 骨骼动画核心算法

**文件**: `animation.rs:178-211`

#### Step 1: 初始化骨骼数据结构

```rust
let mut bones: Vec<_> = skel.bones
    .iter()
    .enumerate()
    .take(MAX_BONE_COUNT)  // 最多512个骨骼
    .map(|(i, b)| {
        (
            i,
            AnimatedBone {
                bone: b,
                compensate_scale: false,
                anim_transform: None,
                flags: TransformFlags::default(),
            },
        )
    })
    .collect();
```

#### Step 2: 应用动画变换

**文件**: `animation.rs:367-389`

```rust
fn apply_transforms(bones: &mut [(usize, AnimatedBone)], anim: &AnimData, frame: f32) {
    for group in &anim.groups {
        if group.group_type == GroupType::Transform {
            for node in &group.nodes {
                // 查找匹配的骨骼
                if let Some((_, bone)) = bones.iter_mut().find(|(_, b)| b.bone.name == node.name) {
                    if let Some(track) = node.tracks.first() {
                        if let TrackValues::Transform(values) = &track.values {
                            *bone = create_animated_bone(frame, bone.bone, track, values);
                        }
                    }
                }
            }
        }
    }
}
```

#### Step 3: 帧插值计算

**文件**: `animation.rs:560-573`

```rust
fn frame_value<T: Interpolate>(values: &[T], frame: f32) -> T {
    // Force the frame to be in bounds.
    let current_frame = (frame.floor() as usize).clamp(0, values.len() - 1);
    let next_frame = (frame.ceil() as usize).clamp(0, values.len() - 1);
    let factor = frame.fract();

    // Frame values like 3.5 should be an average of values[3] and values[4].
    values[current_frame].interpolate(&values[next_frame], factor)
}
```

**关键插值算法**:

1. **线性插值（位置、缩放）**
```rust
impl Interpolate for Vector3 {
    fn interpolate(&self, other: &Self, factor: f32) -> Self {
        glam::Vec3::from(self.to_array())
            .lerp(glam::Vec3::from(other.to_array()), factor)
            .to_array()
            .into()
    }
}
```

2. **四元数插值（旋转）**
```rust
fn interpolate_quat(a: &Vector4, b: &Vector4, factor: f32) -> Vector4 {
    glam::quat(a.x, a.y, a.z, a.w)
        .lerp(glam::quat(b.x, b.y, b.z, b.w), factor)
        .to_array()
        .into()
}

impl Interpolate for Transform {
    fn interpolate(&self, other: &Self, factor: f32) -> Self {
        Self {
            translation: self.translation.interpolate(&other.translation, factor),
            rotation: interpolate_quat(&self.rotation, &other.rotation, factor),
            scale: self.scale.interpolate(&other.scale, factor),
        }
    }
}
```

#### Step 4: 计算世界变换矩阵

**文件**: `animation.rs:213-269`

```rust
pub fn animate_skel_inner(
    result: &mut AnimationTransforms,
    bones: &mut [(usize, AnimatedBone)],
    skel_bones: &[BoneData],
    hlpb: Option<&HlpbData>,
) {
    // 1. 拓扑排序获取评估顺序（父骨骼优先）
    let evaluation_order = evaluation_order(bones);
    
    // 2. 计算世界变换
    for i in &evaluation_order {
        let bone = &bones[*i];
        let (parent_world, current) = calculate_world_transform(bones, &bone.1, result);
        result.world_transforms[bone.0] = parent_world * current;
    }
    
    // 3. 应用Helper Bone约束（可选）
    if let Some(hlpb) = hlpb {
        for i in &evaluation_order {
            let bone = &bones[*i];
            let (parent_world, mut current) = calculate_world_transform(bones, &bone.1, result);
            
            apply_constraints(&mut current, hlpb, bone, result, skel_bones);
            
            result.world_transforms[bone.0] = parent_world * current;
        }
    }
}
```

**父子骨骼变换合成**:

**文件**: `animation.rs:332-365`

```rust
fn calculate_world_transform(
    bones: &[(usize, AnimatedBone)],
    bone: &AnimatedBone,
    result: &AnimationTransforms,
) -> (glam::Mat4, glam::Mat4) {
    if let Some(parent_index) = bone.bone.parent_index {
        let parent_transform = result.world_transforms[parent_index];
        
        // 缩放补偿处理
        let scale_compensation = if bone.compensate_scale {
            let parent_scale = bones[parent_index]
                .1
                .anim_transform
                .map(|t| t.scale)
                .unwrap_or(glam::Vec3::ONE);
            1.0 / parent_scale
        } else {
            glam::Vec3::ONE
        };
        
        let current_transform = bone.animated_transform(scale_compensation);
        (parent_transform, current_transform)
    } else {
        // 根骨骼
        (glam::Mat4::IDENTITY, bone.animated_transform(glam::Vec3::ONE))
    }
}
```

#### Step 5: 变换矩阵构建

**文件**: `animation.rs:30-68`

```rust
impl AnimatedBone {
    fn animated_transform(&self, scale_compensation: glam::Vec3) -> glam::Mat4 {
        self.anim_transform
            .as_ref()
            .map(|t| {
                // 应用TransformFlags覆盖
                let (skel_scale, skel_rot, skel_trans) =
                    glam::Mat4::from_cols_array_2d(&self.bone.transform)
                        .to_scale_rotation_translation();
                
                let adjusted_transform = AnimTransform {
                    translation: if self.flags.override_translation {
                        skel_trans
                    } else {
                        t.translation
                    },
                    rotation: if self.flags.override_rotation {
                        skel_rot
                    } else {
                        t.rotation
                    },
                    scale: if self.flags.override_scale {
                        skel_scale
                    } else {
                        t.scale
                    },
                };
                
                adjusted_transform.to_mat4(scale_compensation)
            })
            .unwrap_or_else(|| glam::Mat4::from_cols_array_2d(&self.bone.transform))
    }
}

impl AnimTransform {
    fn to_mat4(self, scale_compensation: glam::Vec3) -> glam::Mat4 {
        let translation = glam::Mat4::from_translation(self.translation);
        let rotation = glam::Mat4::from_quat(self.rotation);
        let scale = glam::Mat4::from_scale(self.scale);
        // 应用顺序: scale -> rotation -> compensation -> translation
        translation * glam::Mat4::from_scale(scale_compensation) * rotation * scale
    }
}
```

### 3. 关键技术要点总结

#### a) 帧率标准
- 大乱斗动画：**60 FPS**
- frame索引可以是浮点数（如5.7），用于插值

#### b) 插值类型
- **位置和缩放**: 线性插值 (lerp)
- **旋转**: 四元数线性插值 (quat lerp)
- **布尔值**: 不插值，直接取值

#### c) 缩放补偿 (Compensate Scale)
当子骨骼不希望继承父骨骼的缩放时：
```
scale_compensation = 1.0 / parent_scale
final_scale = child_scale * scale_compensation
```

#### d) 变换标志 (TransformFlags)
允许动画覆盖骨骼默认变换的特定部分：
- `override_translation`: 使用骨骼的默认位置而非动画位置
- `override_rotation`: 使用骨骼的默认旋转而非动画旋转
- `override_scale`: 使用骨骼的默认缩放而非动画缩放

#### e) 评估顺序
使用拓扑排序确保父骨骼在子骨骼之前计算：
```
Root -> Child1 -> Grandchild1
     -> Child2 -> Grandchild2
```

---

## Maya Anim文件格式

### 1. Maya Anim文件基础

Maya的 `.anim` 文件是ASCII格式的关键帧动画文件。

#### 文件结构示例

```
animVersion 1.1;
mayaVersion 2020;
timeUnit ntsc;    // 29.97 fps
linearUnit cm;
angularUnit deg;

// 动画曲线定义
anim rotate.rotateX rotateX joint1 0 1 1;
  animData {
    input time;
    output angular;
    weighted 0;
    keys {
      0 0 fixed fixed 1 1 1;
      10 45 fixed fixed 1 1 1;
      20 90 fixed fixed 1 1 1;
    }
  }

anim translate.translateX translateX joint1 0 1 2;
  animData {
    input time;
    output linear;
    weighted 0;
    keys {
      0 0 fixed fixed 1 1 1;
      10 5 fixed fixed 1 1 1;
      20 10 fixed fixed 1 1 1;
    }
  }
```

### 2. 关键字说明

#### 头部声明
```
animVersion 1.1;           // Maya动画版本
mayaVersion 2020;          // Maya软件版本
timeUnit ntsc;             // 时间单位 (film=24fps, ntsc=29.97fps, pal=25fps, game=15fps, etc.)
linearUnit cm;             // 线性单位 (mm, cm, m, etc.)
angularUnit deg;           // 角度单位 (deg, rad)
```

#### 动画曲线语法
```
anim <attribute_path> <attribute_name> <object_name> <input_type> <output_type> <index>;
```

参数说明：
- `attribute_path`: 属性路径（如 `translate.translateX`, `rotate.rotateY`）
- `attribute_name`: 属性简称
- `object_name`: 目标骨骼/对象名称
- `input_type`: 输入类型（通常为0，表示时间）
- `output_type`: 输出类型（1=角度, 2=线性）
- `index`: 曲线索引

#### Keys语法
```
keys {
  <frame> <value> <in_tangent_type> <out_tangent_type> <lock> <weight_lock> <breakdown>;
}
```

参数说明：
- `frame`: 帧号
- `value`: 关键帧值
- `in_tangent_type`: 入切线类型（`fixed`, `linear`, `flat`, `step`, `slow`, `fast`, `spline`, `clamped`, `plateau`, `stepnext`）
- `out_tangent_type`: 出切线类型
- `lock`: 切线锁定（1=锁定, 0=不锁定）
- `weight_lock`: 权重锁定
- `breakdown`: 是否为breakdown帧

### 3. 属性映射

NUANMB → Maya映射：

| NUANMB属性 | Maya属性路径 | 输出类型 | 单位 |
|-----------|-------------|---------|-----|
| translation.x | translate.translateX | 2 (linear) | cm |
| translation.y | translate.translateY | 2 (linear) | cm |
| translation.z | translate.translateZ | 2 (linear) | cm |
| rotation (四元数) | rotate.rotateX | 1 (angular) | deg |
| rotation (四元数) | rotate.rotateY | 1 (angular) | deg |
| rotation (四元数) | rotate.rotateZ | 1 (angular) | deg |
| scale.x | scale.scaleX | 2 (linear) | - |
| scale.y | scale.scaleY | 2 (linear) | - |
| scale.z | scale.scaleZ | 2 (linear) | - |

### 4. 完整示例

假设有一个骨骼 `joint1` 在3帧的动画：

```
animVersion 1.1;
mayaVersion 2020;
timeUnit film;
linearUnit cm;
angularUnit deg;

// Translation X
anim translate.translateX translateX joint1 0 2 1;
  animData {
    input time;
    output linear;
    weighted 0;
    keys {
      0 0.0 fixed fixed 1 1 0;
      1 5.0 fixed fixed 1 1 0;
      2 10.0 fixed fixed 1 1 0;
    }
  }

// Rotation X (从四元数转换的欧拉角)
anim rotate.rotateX rotateX joint1 0 1 2;
  animData {
    input time;
    output angular;
    weighted 0;
    keys {
      0 0.0 fixed fixed 1 1 0;
      1 45.0 fixed fixed 1 1 0;
      2 90.0 fixed fixed 1 1 0;
    }
  }

// ... 其他属性类似
```

---

## Python转换逻辑设计

### 1. 项目结构

```
nuanmb_to_maya/
├── src/
│   ├── __init__.py
│   ├── converter.py          # 主转换器
│   ├── nuanmb_parser.py      # NUANMB JSON解析
│   ├── maya_writer.py        # Maya Anim文件写入
│   ├── math_utils.py         # 数学工具（四元数转欧拉角等）
│   └── models.py             # 数据模型
├── tests/
│   ├── test_parser.py
│   ├── test_converter.py
│   └── test_math.py
├── examples/
│   ├── sample_input.json
│   └── sample_output.anim
├── requirements.txt
├── setup.py
└── README.md
```

### 2. 核心类设计

#### models.py - 数据模型

```python
from dataclasses import dataclass
from typing import List, Optional
from enum import Enum

class GroupType(Enum):
    TRANSFORM = "Transform"
    VISIBILITY = "Visibility"
    MATERIAL = "Material"

@dataclass
class Vector3:
    x: float
    y: float
    z: float
    
    def to_list(self) -> List[float]:
        return [self.x, self.y, self.z]

@dataclass
class Vector4:
    x: float
    y: float
    z: float
    w: float
    
    def to_list(self) -> List[float]:
        return [self.x, self.y, self.z, self.w]

@dataclass
class Transform:
    translation: Vector3
    rotation: Vector4  # Quaternion
    scale: Vector3

@dataclass
class TransformFlags:
    override_translation: bool = False
    override_rotation: bool = False
    override_scale: bool = False
    override_compensate_scale: bool = False

@dataclass
class Track:
    name: str
    compensate_scale: bool
    transform_flags: TransformFlags
    values: List[Transform]  # Frame-by-frame data

@dataclass
class Node:
    name: str  # Bone name
    tracks: List[Track]

@dataclass
class Group:
    group_type: GroupType
    nodes: List[Node]

@dataclass
class AnimData:
    major_version: int
    minor_version: int
    final_frame_index: float
    groups: List[Group]

@dataclass
class MayaKeyframe:
    frame: int
    value: float
    in_tangent: str = "fixed"
    out_tangent: str = "fixed"
    lock: int = 1
    weight_lock: int = 1
    breakdown: int = 0

@dataclass
class MayaAnimCurve:
    attribute_path: str
    attribute_name: str
    object_name: str
    input_type: int
    output_type: int  # 1=angular, 2=linear
    index: int
    keys: List[MayaKeyframe]
```

#### math_utils.py - 数学工具

```python
import numpy as np
from typing import Tuple
from .models import Vector3, Vector4

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
        sy = np.sqrt(rot_mat[0,0]**2 + rot_mat[1,0]**2)
        
        singular = sy < 1e-6
        
        if not singular:
            x = np.arctan2(rot_mat[2,1], rot_mat[2,2])
            y = np.arctan2(-rot_mat[2,0], sy)
            z = np.arctan2(rot_mat[1,0], rot_mat[0,0])
        else:
            x = np.arctan2(-rot_mat[1,2], rot_mat[1,1])
            y = np.arctan2(-rot_mat[2,0], sy)
            z = 0
        
        return Vector3(
            x=np.degrees(x),
            y=np.degrees(y),
            z=np.degrees(z)
        )
    else:
        raise NotImplementedError(f"Rotation order {order} not implemented")

def lerp_vector3(a: Vector3, b: Vector3, t: float) -> Vector3:
    """Linear interpolation for Vector3"""
    return Vector3(
        x=a.x * (1 - t) + b.x * t,
        y=a.y * (1 - t) + b.y * t,
        z=a.z * (1 - t) + b.z * t
    )

def slerp_quat(a: Vector4, b: Vector4, t: float) -> Vector4:
    """Spherical linear interpolation for quaternions"""
    # Normalize quaternions
    a_arr = np.array([a.x, a.y, a.z, a.w])
    b_arr = np.array([b.x, b.y, b.z, b.w])
    a_arr = a_arr / np.linalg.norm(a_arr)
    b_arr = b_arr / np.linalg.norm(b_arr)
    
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
    """Interpolate between two transforms"""
    return Transform(
        translation=lerp_vector3(a.translation, b.translation, t),
        rotation=slerp_quat(a.rotation, b.rotation, t),
        scale=lerp_vector3(a.scale, a.scale, t)
    )
```

#### nuanmb_parser.py - JSON解析器

```python
import json
from typing import List, Dict, Any
from .models import *

class NuanmbParser:
    """Parse NUANMB JSON data"""
    
    def __init__(self, json_path: str):
        self.json_path = json_path
        self.anim_data: Optional[AnimData] = None
    
    def parse(self) -> AnimData:
        """Parse JSON file and return AnimData"""
        with open(self.json_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        self.anim_data = self._parse_anim_data(data)
        return self.anim_data
    
    def _parse_anim_data(self, data: Dict[str, Any]) -> AnimData:
        """Parse root AnimData structure"""
        return AnimData(
            major_version=data['major_version'],
            minor_version=data['minor_version'],
            final_frame_index=data['final_frame_index'],
            groups=[self._parse_group(g) for g in data['groups']]
        )
    
    def _parse_group(self, data: Dict[str, Any]) -> Group:
        """Parse Group structure"""
        return Group(
            group_type=GroupType(data['group_type']),
            nodes=[self._parse_node(n) for n in data['nodes']]
        )
    
    def _parse_node(self, data: Dict[str, Any]) -> Node:
        """Parse Node structure"""
        return Node(
            name=data['name'],
            tracks=[self._parse_track(t) for t in data['tracks']]
        )
    
    def _parse_track(self, data: Dict[str, Any]) -> Track:
        """Parse Track structure"""
        # Parse values based on type
        values_data = data['values']
        
        if 'Transform' in values_data:
            values = [self._parse_transform(t) for t in values_data['Transform']]
        else:
            # Handle other value types if needed
            values = []
        
        return Track(
            name=data['name'],
            compensate_scale=data['compensate_scale'],
            transform_flags=self._parse_transform_flags(data['transform_flags']),
            values=values
        )
    
    def _parse_transform(self, data: Dict[str, Any]) -> Transform:
        """Parse Transform structure"""
        return Transform(
            translation=Vector3(**data['translation']),
            rotation=Vector4(**data['rotation']),
            scale=Vector3(**data['scale'])
        )
    
    def _parse_transform_flags(self, data: Dict[str, Any]) -> TransformFlags:
        """Parse TransformFlags structure"""
        return TransformFlags(**data)
```

#### maya_writer.py - Maya文件写入器

```python
from typing import List, TextIO
from .models import *

class MayaAnimWriter:
    """Write Maya .anim files"""
    
    def __init__(self, output_path: str):
        self.output_path = output_path
        self.curves: List[MayaAnimCurve] = []
    
    def add_curve(self, curve: MayaAnimCurve):
        """Add an animation curve"""
        self.curves.append(curve)
    
    def write(self):
        """Write all curves to file"""
        with open(self.output_path, 'w', encoding='utf-8') as f:
            self._write_header(f)
            
            for curve in self.curves:
                self._write_curve(f, curve)
    
    def _write_header(self, f: TextIO):
        """Write Maya anim file header"""
        f.write("animVersion 1.1;\n")
        f.write("mayaVersion 2020;\n")
        f.write("timeUnit ntsc;  // Convert from 60fps to 29.97fps\n")
        f.write("linearUnit cm;\n")
        f.write("angularUnit deg;\n")
        f.write("\n")
    
    def _write_curve(self, f: TextIO, curve: MayaAnimCurve):
        """Write a single animation curve"""
        f.write(f"anim {curve.attribute_path} {curve.attribute_name} ")
        f.write(f"{curve.object_name} {curve.input_type} {curve.output_type} {curve.index};\n")
        f.write("  animData {\n")
        f.write("    input time;\n")
        
        if curve.output_type == 1:
            f.write("    output angular;\n")
        else:
            f.write("    output linear;\n")
        
        f.write("    weighted 0;\n")
        f.write("    keys {\n")
        
        for key in curve.keys:
            f.write(f"      {key.frame} {key.value} ")
            f.write(f"{key.in_tangent} {key.out_tangent} ")
            f.write(f"{key.lock} {key.weight_lock} {key.breakdown};\n")
        
        f.write("    }\n")
        f.write("  }\n")
        f.write("\n")
```

#### converter.py - 主转换器

```python
from typing import List, Dict
from .nuanmb_parser import NuanmbParser
from .maya_writer import MayaAnimWriter
from .math_utils import quat_to_euler
from .models import *

class NuanmbToMayaConverter:
    """Convert NUANMB animation to Maya .anim format"""
    
    def __init__(self, input_json: str, output_anim: str):
        self.input_json = input_json
        self.output_anim = output_anim
        self.parser = NuanmbParser(input_json)
        self.writer = MayaAnimWriter(output_anim)
        self.fps_conversion = 29.97 / 60.0  # Convert from 60fps to Maya's ntsc (29.97fps)
    
    def convert(self):
        """Main conversion process"""
        # Step 1: Parse NUANMB JSON
        anim_data = self.parser.parse()
        
        # Step 2: Extract Transform groups only (骨骼动画)
        transform_groups = [g for g in anim_data.groups 
                          if g.group_type == GroupType.TRANSFORM]
        
        # Step 3: Process each bone
        for group in transform_groups:
            for node in group.nodes:
                self._process_bone(node, anim_data.final_frame_index)
        
        # Step 4: Write Maya file
        self.writer.write()
        
        print(f"Conversion complete: {self.output_anim}")
        print(f"Total curves: {len(self.writer.curves)}")
    
    def _process_bone(self, node: Node, final_frame: float):
        """Process a single bone's animation"""
        bone_name = node.name
        
        # Find transform track
        transform_track = None
        for track in node.tracks:
            if track.name == "Transform" and len(track.values) > 0:
                transform_track = track
                break
        
        if not transform_track:
            return
        
        # Generate curves for each component
        curve_index = 0
        
        # Translation
        for axis, attr in [('x', 'translateX'), ('y', 'translateY'), ('z', 'translateZ')]:
            keys = self._create_translation_keys(transform_track, axis, final_frame)
            curve = MayaAnimCurve(
                attribute_path=f"translate.{attr}",
                attribute_name=attr,
                object_name=bone_name,
                input_type=0,
                output_type=2,  # Linear
                index=curve_index,
                keys=keys
            )
            self.writer.add_curve(curve)
            curve_index += 1
        
        # Rotation (convert quaternion to Euler)
        euler_keys = self._create_rotation_keys(transform_track, final_frame)
        
        for axis, attr, keys in [
            ('x', 'rotateX', euler_keys['x']),
            ('y', 'rotateY', euler_keys['y']),
            ('z', 'rotateZ', euler_keys['z'])
        ]:
            curve = MayaAnimCurve(
                attribute_path=f"rotate.{attr}",
                attribute_name=attr,
                object_name=bone_name,
                input_type=0,
                output_type=1,  # Angular
                index=curve_index,
                keys=keys
            )
            self.writer.add_curve(curve)
            curve_index += 1
        
        # Scale
        for axis, attr in [('x', 'scaleX'), ('y', 'scaleY'), ('z', 'scaleZ')]:
            keys = self._create_scale_keys(transform_track, axis, final_frame)
            curve = MayaAnimCurve(
                attribute_path=f"scale.{attr}",
                attribute_name=attr,
                object_name=bone_name,
                input_type=0,
                output_type=2,  # Linear
                index=curve_index,
                keys=keys
            )
            self.writer.add_curve(curve)
            curve_index += 1
    
    def _create_translation_keys(self, track: Track, axis: str, 
                                 final_frame: float) -> List[MayaKeyframe]:
        """Create translation keyframes for a specific axis"""
        keys = []
        values = track.values
        
        for frame_idx, transform in enumerate(values):
            # Convert frame from 60fps to Maya fps
            maya_frame = int(frame_idx * self.fps_conversion)
            
            # Get value for the axis
            if axis == 'x':
                value = transform.translation.x
            elif axis == 'y':
                value = transform.translation.y
            else:
                value = transform.translation.z
            
            keys.append(MayaKeyframe(
                frame=maya_frame,
                value=value
            ))
        
        return keys
    
    def _create_rotation_keys(self, track: Track, final_frame: float) -> Dict[str, List[MayaKeyframe]]:
        """Create rotation keyframes (convert quaternion to Euler)"""
        euler_keys = {'x': [], 'y': [], 'z': []}
        values = track.values
        
        for frame_idx, transform in enumerate(values):
            # Convert frame from 60fps to Maya fps
            maya_frame = int(frame_idx * self.fps_conversion)
            
            # Convert quaternion to Euler angles
            euler = quat_to_euler(transform.rotation, order='XYZ')
            
            euler_keys['x'].append(MayaKeyframe(frame=maya_frame, value=euler.x))
            euler_keys['y'].append(MayaKeyframe(frame=maya_frame, value=euler.y))
            euler_keys['z'].append(MayaKeyframe(frame=maya_frame, value=euler.z))
        
        return euler_keys
    
    def _create_scale_keys(self, track: Track, axis: str, 
                          final_frame: float) -> List[MayaKeyframe]:
        """Create scale keyframes for a specific axis"""
        keys = []
        values = track.values
        
        for frame_idx, transform in enumerate(values):
            # Convert frame from 60fps to Maya fps
            maya_frame = int(frame_idx * self.fps_conversion)
            
            # Get value for the axis
            if axis == 'x':
                value = transform.scale.x
            elif axis == 'y':
                value = transform.scale.y
            else:
                value = transform.scale.z
            
            keys.append(MayaKeyframe(
                frame=maya_frame,
                value=value
            ))
        
        return keys
```

---

## 核心算法实现

### 1. 主程序入口

```python
# main.py
import argparse
from src.converter import NuanmbToMayaConverter

def main():
    parser = argparse.ArgumentParser(
        description='Convert NUANMB animation JSON to Maya .anim format'
    )
    parser.add_argument('input', help='Input JSON file (from ssbh_data)')
    parser.add_argument('output', help='Output Maya .anim file')
    parser.add_argument('--fps', type=float, default=29.97,
                       help='Target Maya FPS (default: 29.97 for ntsc)')
    
    args = parser.parse_args()
    
    # Create converter
    converter = NuanmbToMayaConverter(args.input, args.output)
    converter.fps_conversion = args.fps / 60.0
    
    # Run conversion
    converter.convert()

if __name__ == '__main__':
    main()
```

### 2. 使用示例

```bash
# Step 1: 使用ssbh_data将nuanmb转换为JSON
ssbh_data_json animation.nuanmb animation.json

# Step 2: 使用Python工具转换为Maya anim
python main.py animation.json animation.anim

# 可选：指定目标FPS
python main.py animation.json animation.anim --fps 24
```

### 3. 高级功能扩展

#### a) 支持缩放补偿

```python
def apply_scale_compensation(self, node: Node, parent_scale: Vector3) -> Track:
    """Apply scale compensation logic from ssbh_wgpu"""
    track = node.tracks[0]
    
    if track.compensate_scale:
        compensated_values = []
        for transform in track.values:
            # Calculate compensation
            comp_x = 1.0 / parent_scale.x if parent_scale.x != 0 else 1.0
            comp_y = 1.0 / parent_scale.y if parent_scale.y != 0 else 1.0
            comp_z = 1.0 / parent_scale.z if parent_scale.z != 0 else 1.0
            
            compensated_transform = Transform(
                translation=transform.translation,
                rotation=transform.rotation,
                scale=Vector3(
                    x=transform.scale.x * comp_x,
                    y=transform.scale.y * comp_y,
                    z=transform.scale.z * comp_z
                )
            )
            compensated_values.append(compensated_transform)
        
        track.values = compensated_values
    
    return track
```

#### b) 支持TransformFlags

```python
def apply_transform_flags(self, transform: Transform, 
                         flags: TransformFlags,
                         skel_default: Transform) -> Transform:
    """Apply transform override flags"""
    return Transform(
        translation=skel_default.translation if flags.override_translation 
                   else transform.translation,
        rotation=skel_default.rotation if flags.override_rotation 
                else transform.rotation,
        scale=skel_default.scale if flags.override_scale 
             else transform.scale
    )
```

#### c) 骨骼层级处理

如果需要处理骨骼层级关系（从skeleton文件读取）:

```python
@dataclass
class Bone:
    name: str
    parent_index: Optional[int]
    transform: Transform  # Rest pose

class SkeletonProcessor:
    def __init__(self, skel_json: str):
        self.bones: List[Bone] = self._load_skeleton(skel_json)
    
    def calculate_world_transform(self, bone_index: int, 
                                  local_transforms: List[Transform]) -> Transform:
        """Calculate world transform for a bone"""
        bone = self.bones[bone_index]
        local = local_transforms[bone_index]
        
        if bone.parent_index is None:
            return local
        
        # Recursively get parent world transform
        parent_world = self.calculate_world_transform(
            bone.parent_index, local_transforms
        )
        
        # Combine parent world with local
        # This requires matrix multiplication
        return self._combine_transforms(parent_world, local)
```

---

## 开发路线图

### Phase 1: 基础转换 (1-2周)
- [x] 需求分析和技术调研
- [ ] 实现基本数据模型
- [ ] 实现JSON解析器
- [ ] 实现四元数到欧拉角转换
- [ ] 实现Maya文件写入器
- [ ] 完成基础Translation/Rotation/Scale转换

### Phase 2: 高级功能 (1-2周)
- [ ] 实现缩放补偿逻辑
- [ ] 实现TransformFlags处理
- [ ] 实现帧插值（可选，如果需要重采样）
- [ ] 支持多种Maya FPS设置
- [ ] 添加详细的日志输出

### Phase 3: 测试与优化 (1周)
- [ ] 单元测试（每个模块）
- [ ] 集成测试（完整转换流程）
- [ ] 在Maya中验证输出结果
- [ ] 性能优化（大型动画文件）
- [ ] 错误处理和异常捕获

### Phase 4: 文档与发布 (1周)
- [ ] 编写使用文档
- [ ] 添加示例文件
- [ ] 创建教程视频/图文
- [ ] 打包发布

---

## 技术难点与解决方案

### 1. 四元数到欧拉角转换

**难点**: 
- 万向锁问题
- 多种旋转顺序（XYZ, YZX, ZXY等）
- 数值精度

**解决方案**:
- 使用标准的rotation matrix intermediate方法
- 明确指定旋转顺序（Maya默认XYZ）
- 使用numpy进行高精度计算

### 2. 帧率转换

**难点**:
- 大乱斗60fps vs Maya多种fps（24, 29.97, 30等）
- 可能需要重采样

**解决方案**:
```python
# 简单方法：整数映射
maya_frame = int(nuanmb_frame * (maya_fps / 60.0))

# 高质量方法：插值重采样
def resample_animation(values: List[Transform], 
                      source_fps: float, target_fps: float) -> List[Transform]:
    """Resample animation to different frame rate with interpolation"""
    source_duration = len(values) / source_fps
    target_frame_count = int(source_duration * target_fps)
    
    resampled = []
    for i in range(target_frame_count):
        target_time = i / target_fps
        source_frame = target_time * source_fps
        
        # Interpolate
        frame_floor = int(source_frame)
        frame_ceil = min(frame_floor + 1, len(values) - 1)
        t = source_frame - frame_floor
        
        interpolated = interpolate_transform(
            values[frame_floor],
            values[frame_ceil],
            t
        )
        resampled.append(interpolated)
    
    return resampled
```

### 3. 骨骼层级关系

**难点**:
- NUANMB动画只包含局部变换
- 可能需要骨骼层级信息（从.nusktb文件）

**解决方案**:
- Option 1: 只导出局部变换（推荐，简单）
- Option 2: 如果需要世界空间，额外解析skeleton文件

### 4. 可见性和材质动画

**难点**:
- Maya的可见性属性处理
- 材质动画在Maya中的对应

**解决方案**:
```python
# 可见性转换
def convert_visibility(node: Node) -> MayaAnimCurve:
    """Convert boolean visibility to Maya visibility attribute"""
    keys = []
    for frame, value in enumerate(node.tracks[0].values):
        keys.append(MayaKeyframe(
            frame=frame,
            value=1.0 if value else 0.0  # True=1.0, False=0.0
        ))
    
    return MayaAnimCurve(
        attribute_path="visibility",
        attribute_name="visibility",
        object_name=node.name,
        input_type=0,
        output_type=2,  # Linear
        index=0,
        keys=keys
    )
```

---

## 测试策略

### 单元测试

```python
# tests/test_math.py
def test_quat_to_euler_identity():
    q = Vector4(0, 0, 0, 1)  # Identity quaternion
    euler = quat_to_euler(q)
    assert abs(euler.x) < 1e-6
    assert abs(euler.y) < 1e-6
    assert abs(euler.z) < 1e-6

def test_quat_to_euler_90_degrees_x():
    q = Vector4(0.707, 0, 0, 0.707)  # 90 degrees around X
    euler = quat_to_euler(q)
    assert abs(euler.x - 90.0) < 0.1
    assert abs(euler.y) < 0.1
    assert abs(euler.z) < 0.1
```

### 集成测试

```python
# tests/test_converter.py
def test_full_conversion():
    converter = NuanmbToMayaConverter(
        'tests/data/sample.json',
        'tests/output/sample.anim'
    )
    converter.convert()
    
    # Verify output file exists
    assert os.path.exists('tests/output/sample.anim')
    
    # Verify basic content
    with open('tests/output/sample.anim', 'r') as f:
        content = f.read()
        assert 'animVersion 1.1' in content
        assert 'mayaVersion' in content
```

### Maya验证

在Maya中运行MEL脚本验证：

```mel
// 导入生成的anim文件
file -import -type "animImport" -ra true "path/to/output.anim";

// 验证关键帧数量
int $keyCount = `keyframe -query -keyframeCount translateX`;
print("Total keys: " + $keyCount + "\n");

// 播放动画检查
play -forward true;
```

---

## 依赖安装

### requirements.txt

```txt
numpy>=1.21.0
```

### setup.py

```python
from setuptools import setup, find_packages

setup(
    name='nuanmb-to-maya',
    version='0.1.0',
    description='Convert NUANMB animation files to Maya .anim format',
    packages=find_packages(),
    install_requires=[
        'numpy>=1.21.0',
    ],
    python_requires='>=3.8',
    entry_points={
        'console_scripts': [
            'nuanmb2maya=src.main:main',
        ],
    },
)
```

---

## 总结

本文档提供了将NUANMB动画转换为Maya格式的完整开发指南，包括：

1. ✅ **详细的JSON格式分析** - 理解ssbh_data输出
2. ✅ **深入的apply_anims实现分析** - 从ssbh_wgpu学习动画应用逻辑
3. ✅ **完整的Maya文件格式说明** - 了解目标格式
4. ✅ **全面的Python代码架构** - 可直接使用的代码框架
5. ✅ **核心算法实现** - 四元数转换、插值等
6. ✅ **测试和验证策略** - 确保转换质量

开发者可以基于本文档直接开始实现转换工具。

---

**文档版本**: 1.0  
**创建日期**: 2025-10-04  
**作者**: AI Assistant  
**基于**: ssbh_wgpu commit latest, ssbh_data 0.19.0

