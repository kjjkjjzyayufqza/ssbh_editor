# SSBH Editor - NUANMB 动画文件加载流程详解

本文档详细说明了 SSBH Editor 项目如何加载、解析和应用 `.nuanmb` 动画文件到场景中的模型和骨骼。

## 目录

1. [概述](#概述)
2. [文件加载阶段](#文件加载阶段)
3. [数据解析与结构](#数据解析与结构)
4. [动画管理系统](#动画管理系统)
5. [场景应用流程](#场景应用流程)
6. [帧数据更新与渲染](#帧数据更新与渲染)
7. [完整流程图](#完整流程图)

---

## 概述

NUANMB（Nintendo U Animation Binary）是任天堂明星大乱斗特别版（Super Smash Bros. Ultimate）中使用的动画文件格式。本项目通过多个模块协同工作来加载和播放这些动画：

- **文件系统层**：负责从磁盘读取 `.nuanmb` 文件
- **数据解析层**：使用 `ssbh_data` 库将二进制数据转换为结构化数据
- **动画管理层**：管理多个动画槽位和播放状态
- **渲染层**：使用 `ssbh_wgpu` 库将动画数据应用到GPU渲染管线

---

## 文件加载阶段

### 1. 文件夹扫描

当用户通过 "File > Open Folder" 或拖放文件夹时，程序会触发文件加载流程：

**代码位置**：`src/app.rs:633-692`

```rust
pub fn add_folder_to_workspace<P: AsRef<Path>>(&mut self, folder: P, clear_workspace: bool) {
    // Load recursively for nested folders like stages.
    let mut new_models = ssbh_wgpu::load_model_folders(&folder);
    
    // Don't add any folders that have already been added.
    new_models.retain(|(p, _)| !self.models.iter().any(|m| &m.folder_path == p));
    
    // List folders alphabetically.
    new_models.sort_by_key(|(p, _)| p.clone());
    
    // ... (动画槽位初始化)
}
```

### 2. 递归文件发现

`ssbh_wgpu::load_model_folders()` 函数会：
- 递归扫描目标文件夹及其子文件夹
- 识别所有 `.nuanmb` 文件
- 将文件路径与模型文件夹关联

**典型文件结构**：
```
mario/
├── model/body/c00/
│   ├── model.numshb    (mesh)
│   ├── model.numatb    (materials)
│   ├── model.nusktb    (skeleton)
│   └── model.nuanmb    (基础动画)
└── motion/body/c00/
    ├── a00wait1.nuanmb
    ├── a00attack1.nuanmb
    └── ...
```

### 3. 文件读取与解析

每个发现的 `.nuanmb` 文件都会通过 `AnimData::from_file()` 进行解析：

**代码位置**：`src/lib.rs:185`

```rust
AnimData::from_file(path)
    .map_err(|e| {
        error!("Error reading {path:?}: {e}");
        e
    })
    .ok()
```

这个函数来自 `ssbh_data` 库，负责：
- 读取二进制文件内容
- 验证文件格式（magic number, version等）
- 解析为 `AnimData` 结构体

---

## 数据解析与结构

### AnimData 层次结构

解析后的动画数据采用层次化结构：

```
AnimData
├── major_version: u16
├── minor_version: u16
├── final_frame_index: f32           // 动画总帧数
└── groups: Vec<GroupData>           // 动画组列表
    └── GroupData
        ├── group_type: GroupType    // Transform, Visibility, Material等
        └── nodes: Vec<NodeData>     // 节点列表
            └── NodeData
                ├── name: String     // 骨骼名称或材质名称
                └── tracks: Vec<TrackData>
                    └── TrackData
                        ├── name: String
                        ├── compensate_scale: bool
                        ├── transform_flags: TransformFlags
                        └── values: TrackValues
```

### TrackValues 数据类型

**代码位置**：`src/editors/anim.rs:142-250`

动画轨道支持多种数据类型：

#### 1. Transform（变换数据）
用于骨骼动画，每帧包含：
```rust
Transform {
    translation: Vector3 { x, y, z },  // 位置
    rotation: Vector4 { x, y, z, w },  // 四元数旋转
    scale: Vector3 { x, y, z }         // 缩放
}
```

#### 2. UvTransform（UV变换）
用于材质UV动画：
```rust
UvTransform {
    scale_u: f32,
    scale_v: f32,
    rotation: f32,
    translate_u: f32,
    translate_v: f32
}
```

#### 3. Float（浮点数）
用于各种参数动画（如透明度、权重等）

#### 4. Boolean（布尔值）
用于可见性控制等开关型动画

#### 5. Vector4（四维向量）
用于颜色、方向等数据

#### 6. PatternIndex（贴图索引）
用于切换贴图序列动画

### 数据存储方式

**代码位置**：`src/model_folder.rs:7-15`

```rust
pub struct ModelFolderState {
    pub folder_path: PathBuf,
    pub model: ModelFolder,
    // ... 其他字段
}

// ModelFolder 中包含：
pub struct ModelFolder {
    pub anims: Vec<(String, FileResult<AnimData>)>,  // 动画文件列表
    // (文件名, 解析结果)
    // ...
}
```

---

## 动画管理系统

### 动画槽位（Animation Slots）

项目使用动画槽位系统允许多个动画同时播放（如基础动画 + 表情动画）。

**代码位置**：`src/lib.rs:478-504`

```rust
pub struct AnimationSlot {
    pub is_enabled: bool,              // 是否启用此槽位
    pub animation: Option<AnimationIndex>,  // 关联的动画索引
}

pub struct AnimationIndex {
    pub folder_index: usize,           // 模型文件夹索引
    pub anim_index: usize,             // 该文件夹中的动画索引
}
```

### 自动分配默认动画

加载模型文件夹时，系统会自动查找并分配 `model.nuanmb`：

**代码位置**：`src/app.rs:648-665`

```rust
self.animation_state.animations.extend(
    new_models.iter().enumerate().map(|(i, (_, model))| {
        if let Some(anim_index) = model.anims.iter()
            .position(|(f, _)| f == "model.nuanmb")
        {
            // The model.nuanmb always plays, so assign it automatically.
            vec![AnimationSlot {
                is_enabled: true,
                animation: Some(AnimationIndex {
                    folder_index: self.models.len() + i,
                    anim_index,
                }),
            }]
        } else {
            // Add a dummy animation to prompt the user to select one.
            vec![AnimationSlot::new()]
        }
    })
);
```

### 动画状态管理

**代码位置**：`src/lib.rs:450-470`

```rust
pub struct AnimationState {
    pub current_frame: f32,                   // 当前帧（支持小数，用于平滑插值）
    pub is_playing: bool,                     // 是否正在播放
    pub should_loop: bool,                    // 是否循环播放
    pub playback_speed: f32,                  // 播放速度（1.0为正常速度）
    pub should_update_animations: bool,       // 标记需要更新动画
    pub selected_folder: usize,               // UI选中的文件夹
    pub selected_slot: usize,                 // UI选中的槽位
    pub animations: Vec<Vec<AnimationSlot>>,  // 每个模型的动画槽位列表
    pub previous_frame_start: std::time::Instant,  // 上一帧的时间戳
}
```

### 动画文件夹匹配算法

系统使用路径亲和度算法自动匹配模型和动画文件夹：

**代码位置**：`src/model_folder.rs:104-128`

```rust
fn find_folders_by_path_affinity<'a, P: Fn(&'a ModelFolderState) -> bool>(
    model: &ModelFolderState,
    folders: &'a [ModelFolderState],
    predicate: P,
) -> Vec<(usize, &'a ModelFolderState)> {
    let mut folders: Vec<_> = folders
        .iter()
        .enumerate()
        .filter(|(_, m)| predicate(m))
        .collect();

    // Sort in increasing order of affinity with the model folder.
    // The folder affinity is the number of matching path components.
    folders.sort_by_key(|(_, a)| {
        Path::new(&model.folder_path)
            .components()
            .rev()
            .zip(Path::new(&a.folder_path).components().rev())
            .take_while(|(a, b)| a == b)
            .count()
    });
    folders
}
```

**匹配示例**：
- 模型路径：`/mario/model/body/c00`
- 动画路径1：`/mario/motion/body/c00` → 匹配度：3 (c00, body, mario)
- 动画路径2：`/mario/motion/pump/c00` → 匹配度：2 (c00, mario)

系统会优先推荐匹配度高的动画文件夹。

---

## 场景应用流程

### 1. 主更新循环

每帧渲染时，主更新函数会检查是否需要更新动画：

**代码位置**：`src/app/rendering.rs:71-79`

```rust
if self.animation_state.is_playing || self.animation_state.should_update_animations {
    // Update current frame uniform buffer on GPU
    render_state.renderer.update_current_frame(queue, self.animation_state.current_frame);
    
    // Update lighting animation
    render_state.animate_lighting(queue, self.animation_state.current_frame);
    
    // Update camera animation
    self.animate_viewport_camera(render_state, queue, width, height, scale_factor);
    
    // Apply animations to models
    self.animate_models(queue, render_state);
    
    self.animation_state.should_update_animations = false;
}
```

### 2. 收集启用的动画

**代码位置**：`src/app/rendering.rs:114-161`

```rust
pub fn animate_models(&mut self, queue: &wgpu::Queue, render_state: &mut RenderState) {
    for ((render_model, model), model_animations) in render_state
        .render_models
        .iter_mut()
        .zip(self.models.iter())
        .zip(self.animation_state.animations.iter())
    {
        // Only render enabled animations.
        let animations = model_animations
            .iter()
            .filter(|anim_slot| anim_slot.is_enabled)  // 过滤启用的槽位
            .filter_map(|anim_slot| {
                anim_slot
                    .animation
                    .and_then(|anim_index| anim_index.get_animation(&self.models))
                    .and_then(|(_, a)| a.as_ref())
            });

        render_model.apply_anims(
            queue,
            animations,                // 动画数据迭代器
            skel,                      // 骨骼数据（model.nusktb）
            matl,                      // 材质数据（model.numatb）
            hlpb,                      // 辅助骨骼约束（model.nuhlpb）
            &render_state.shared_data, // 共享渲染数据
            self.animation_state.current_frame,  // 当前帧
        );
    }
}
```

### 3. 应用动画数据到渲染模型

`render_model.apply_anims()` 函数（来自 `ssbh_wgpu` 库）执行以下操作：

#### a) 骨骼动画处理

1. **读取Transform轨道数据**：
   - 遍历所有 `GroupType::Transform` 组
   - 根据 `current_frame` 查找或插值对应帧的变换数据
   
2. **计算骨骼世界变换**：
   ```
   对于每个骨骼：
     1. 从动画中获取局部变换（translation, rotation, scale）
     2. 应用 transform_flags（覆盖标志）
     3. 如果有父骨骼，则合成世界变换：
        world_transform = parent_world_transform * local_transform
     4. 如果启用 compensate_scale，则进行缩放补偿
   ```

3. **更新GPU缓冲区**：
   - 将计算好的骨骼变换矩阵上传到GPU Uniform Buffer
   - 用于顶点着色器中的蒙皮计算

#### b) 材质动画处理

处理 `GroupType::Material` 组：
- **UV Transform动画**：更新材质的UV变换矩阵
- **Float参数动画**：如透明度、金属度等材质参数
- **Vector4动画**：如自发光颜色
- **PatternIndex动画**：切换贴图序列

#### c) 可见性动画处理

处理 `GroupType::Visibility` 组：
- 根据Boolean轨道数据控制网格的可见性
- 每个网格对象可以独立控制显示/隐藏

#### d) 辅助骨骼约束

如果启用了Helper Bones（.nuhlpb文件）：
- 应用IK（逆向运动学）约束
- 应用AimConstraint（瞄准约束）
- 应用OrientConstraint（朝向约束）

这对于预览使用EXO Skel方法的动画Mod非常重要。

---

## 帧数据更新与渲染

### 帧插值计算

动画使用 `current_frame` 的小数部分进行帧间插值，实现平滑动画：

```rust
// 假设 current_frame = 5.7
// 需要在第5帧和第6帧之间插值

fn interpolate_transform(frame: f32, track: &TrackData) -> Transform {
    let frame_floor = frame.floor() as usize;
    let frame_ceil = frame.ceil() as usize;
    let t = frame.fract();  // 0.7
    
    let transform_a = track.values[frame_floor];
    let transform_b = track.values[frame_ceil];
    
    Transform {
        translation: lerp(transform_a.translation, transform_b.translation, t),
        rotation: slerp(transform_a.rotation, transform_b.rotation, t),  // 球面插值
        scale: lerp(transform_a.scale, transform_b.scale, t),
    }
}
```

### 播放控制

**代码位置**：`src/app.rs:1011-1026`

```rust
if self.animation_state.is_playing {
    let final_frame_index = self.max_final_frame_index(render_state);

    self.animation_state.current_frame = next_frame(
        self.animation_state.current_frame,
        current_frame_start.duration_since(self.animation_state.previous_frame_start),
        final_frame_index,
        self.animation_state.playback_speed,
        self.animation_state.should_loop,
    );
    
    // eframe is reactive by default, so we need to repaint.
    ctx.request_repaint();
}

// Always update the frame times even if no animation is playing.
self.animation_state.previous_frame_start = current_frame_start;
```

### 帧率计算

`next_frame()` 函数基于实际经过的时间和播放速度计算下一帧：

```rust
fn next_frame(
    current: f32,
    elapsed: Duration,
    final_frame: f32,
    speed: f32,
    should_loop: bool,
) -> f32 {
    // 大乱斗动画以60 FPS运行
    const TARGET_FPS: f32 = 60.0;
    let frame_delta = elapsed.as_secs_f32() * TARGET_FPS * speed;
    
    let next = current + frame_delta;
    
    if next > final_frame {
        if should_loop {
            next % final_frame  // 循环播放
        } else {
            final_frame  // 停在最后一帧
        }
    } else {
        next
    }
}
```

### 多动画合成规则

当多个动画槽位同时启用时，按以下规则合成：

**代码位置**：`src/app/anim_list.rs:45-46`

```
Slot 0 → 基础动画（model.nuanmb或待机动画）
Slot 1 → 表情动画（如眨眼）
Slot 2 → 额外动画
...

渲染顺序：从Slot 0到Slot N依次应用
后面的Slot会覆盖前面Slot中相同骨骼/材质的动画数据
```

例如：
- Slot 0: `a00wait1.nuanmb` (全身待机动画)
- Slot 1: `a00defaulteyelid.nuanmb` (只影响眼睛骨骼的眨眼动画)

结果：身体使用Slot 0的动画，眼睛使用Slot 1的动画。

### GPU渲染管线

最终，动画数据通过以下管线应用到渲染：

```
1. CPU端计算骨骼变换矩阵
   ↓
2. 上传到GPU Uniform Buffer (update_current_frame)
   ↓
3. 顶点着色器读取骨骼矩阵
   ↓
4. 顶点蒙皮计算：
   transformed_position = Σ(bone_matrix[i] * vertex_position * weight[i])
   ↓
5. 应用材质动画参数
   ↓
6. 片段着色器计算最终颜色
   ↓
7. 输出到屏幕
```

---

## 完整流程图

```
┌─────────────────────────────────────────────────────────────┐
│                     用户操作                                  │
│              File > Open Folder / 拖放文件夹                  │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  文件系统扫描                                 │
│  ssbh_wgpu::load_model_folders()                            │
│  - 递归扫描文件夹                                             │
│  - 发现 *.nuanmb 文件                                        │
│  - 关联到 ModelFolder                                        │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  二进制文件解析                               │
│  AnimData::from_file(path)                                  │
│  - 验证文件格式                                              │
│  - 解析为结构化数据                                           │
│  - 构建 Groups → Nodes → Tracks → Values 层次结构            │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  数据存储                                    │
│  ModelFolderState.model.anims                               │
│  Vec<(String, Option<AnimData>)>                            │
│  [(文件名1, AnimData1), (文件名2, AnimData2), ...]           │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  动画槽位管理                                 │
│  AnimationState.animations                                  │
│  - 自动分配 model.nuanmb 到 Slot 0                          │
│  - 用户通过UI选择其他动画                                     │
│  - 支持多槽位同时播放                                         │
└──────────────────────┬──────────────────────────────────────┘
                       │
        ┌──────────────┴──────────────┐
        │  主渲染循环（每帧）           │
        └──────────────┬──────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  帧数据计算                                   │
│  if is_playing:                                             │
│    current_frame += delta_time * 60.0 * playback_speed      │
│  update_current_frame(queue, current_frame)                 │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  收集启用的动画                               │
│  animate_models()                                           │
│  - 遍历所有模型                                              │
│  - 过滤 is_enabled 的动画槽位                                │
│  - 获取对应的 AnimData                                       │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  应用动画到渲染模型                           │
│  render_model.apply_anims()                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 1. 骨骼动画处理                                      │   │
│  │    - 读取Transform轨道                               │   │
│  │    - 帧插值（lerp, slerp）                          │   │
│  │    - 计算世界变换矩阵                                │   │
│  │    - 应用辅助骨骼约束                                │   │
│  │    - 更新GPU Uniform Buffer                         │   │
│  └─────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 2. 材质动画处理                                      │   │
│  │    - UV Transform                                   │   │
│  │    - 材质参数（Float, Vector4）                      │   │
│  │    - 贴图序列（PatternIndex）                        │   │
│  └─────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 3. 可见性处理                                        │   │
│  │    - Boolean轨道控制网格显示/隐藏                     │   │
│  └─────────────────────────────────────────────────────┘   │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  GPU渲染管线                                 │
│  1. 顶点着色器                                               │
│     - 读取骨骼变换矩阵                                        │
│     - 顶点蒙皮计算（Skinning）                               │
│     - 应用MVP变换                                            │
│                                                             │
│  2. 片段着色器                                               │
│     - 应用材质动画参数                                        │
│     - 采样动画化的UV坐标                                      │
│     - 计算光照和颜色                                          │
│                                                             │
│  3. 输出到屏幕                                               │
└─────────────────────────────────────────────────────────────┘
```

---

## 关键代码文件索引

| 文件路径 | 主要功能 |
|---------|---------|
| `src/app.rs` | 主应用逻辑，文件夹加载，播放控制 |
| `src/app/rendering.rs` | 动画应用到渲染模型 |
| `src/app/anim_list.rs` | 动画列表UI，槽位管理 |
| `src/editors/anim.rs` | 动画编辑器，数据可视化 |
| `src/model_folder.rs` | 文件夹管理，动画匹配算法 |
| `src/lib.rs` | 核心数据结构定义 |
| External: `ssbh_data` | 二进制文件解析库 |
| External: `ssbh_wgpu` | GPU渲染和动画应用库 |

---

## 技术要点总结

### 1. 动画数据是分层存储的
- Groups → Nodes → Tracks → Values
- 支持骨骼、材质、可见性等多种动画类型

### 2. 使用动画槽位系统
- 允许多个动画同时播放和叠加
- 按槽位顺序依次应用，后者覆盖前者

### 3. 智能文件夹匹配
- 基于路径亲和度算法
- 自动关联模型和动画文件夹

### 4. 帧间平滑插值
- 使用浮点数表示当前帧
- 线性插值（位置、缩放）和球面插值（旋转）

### 5. 实时更新机制
- 基于实际经过时间计算帧增量
- 固定60 FPS动画标准
- 支持变速播放

### 6. GPU加速渲染
- 骨骼变换矩阵上传到Uniform Buffer
- 顶点着色器执行蒙皮计算
- 高效处理大量顶点和骨骼

---

## 扩展阅读

- [SSBH Editor Wiki](https://github.com/ScanMountGoat/ssbh_editor/wiki)
- [Anim Editor Wiki](https://github.com/ScanMountGoat/ssbh_editor/wiki/Anim-Editor)
- [Validation Errors](https://github.com/ScanMountGoat/ssbh_editor/wiki/Validation-Errors)
- [ssbh_data 库文档](https://docs.rs/ssbh_data/)
- [ssbh_wgpu 库源码](https://github.com/ScanMountGoat/ssbh_wgpu)

---

**文档版本**：基于 SSBH Editor v0.10.9  
**最后更新**：2025-10-04  
**作者**：AI Assistant

