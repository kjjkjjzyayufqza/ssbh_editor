# SSBH Mesh数据Hex分析报告

## 概述

本文档分析了SSBH v1.8格式中MeshData写入时生成的hex数据结构，特别是AttributeV8数组的二进制表示形式。

## 分析的Hex数据

```
00 00 00 00 34 03 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
01 00 00 00 34 03 00 00 00 00 00 00 0C 00 00 00 00 00 00 00 
03 00 00 00 34 03 00 00 00 00 00 00 18 00 00 00 01 00 00 00 
03 00 00 00 34 03 00 00 00 00 00 00 24 00 00 00 02 00 00 00 
03 00 00 00 34 03 00 00 00 00 00 00 30 00 00 00 03 00 00 00 
04 00 00 00 37 04 00 00 01 00 00 00 00 00 00 00 00 00 00 00 
04 00 00 00 34 04 00 00 01 00 00 00 08 00 00 00 01 00 00 00 
05 00 00 00 37 04 00 00 01 00 00 00 20 00 00 00 00 00 00 00
```

## AttributeV8结构定义

根据ssbh_lib库的定义，每个AttributeV8结构体包含以下字段：

```rust
pub struct AttributeV8 {
    pub usage: AttributeUsageV8,        // 4字节 - 属性用途
    pub data_type: AttributeDataTypeV8, // 4字节 - 数据类型
    pub buffer_index: u32,              // 4字节 - 缓冲区索引
    pub buffer_offset: u32,             // 4字节 - 缓冲区偏移
    pub subindex: u32,                  // 4字节 - 子索引
}
```

总计：20字节 × 7个属性 = 140字节

## 枚举值定义

### AttributeUsageV8枚举

```rust
pub enum AttributeUsageV8 {
    Position = 0,           // 位置
    Normal = 1,             // 法线
    Binormal = 2,           // 副法线 ← 对应 02 00 00 00
    Tangent = 3,            // 切线 ← 对应 03 00 00 00
    TextureCoordinate = 4,  // 纹理坐标
    ColorSet = 5,           // 颜色集
    Unk6 = 6,
    Unk7 = 7,
    HalfFloat2 = 8,
    Unk9 = 9,
}
```

### AttributeDataTypeV8枚举

```rust
pub enum AttributeDataTypeV8 {
    Float3 = 820,        // 0x334 ← 对应 34 03 00 00
    Float4 = 1076,       // 0x434 ← 对应 34 04 00 00
    HalfFloat4 = 1077,   // 0x435 ← 对应 35 04 00 00
    Float2 = 1079,       // 0x437 ← 对应 37 04 00 00
    Byte4 = 1024,
}
```

## 详细属性分析

### 属性1: Position0 (位置)
- **usage**: `00 00 00 00` = 0 (Position)
- **data_type**: `34 03 00 00` = 820 (Float3)
- **buffer_index**: `00 00 00 00` = 0 (buffer0)
- **buffer_offset**: `00 00 00 00` = 0字节
- **subindex**: `00 00 00 00` = 0

### 属性2: Normal0 (法线)
- **usage**: `01 00 00 00` = 1 (Normal)
- **data_type**: `34 03 00 00` = 820 (Float3)
- **buffer_index**: `00 00 00 00` = 0 (buffer0)
- **buffer_offset**: `0C 00 00 00` = 12字节 (3个float × 4字节)
- **subindex**: `00 00 00 00` = 0

### 属性3: Binormal0 (副法线)
- **usage**: `02 00 00 00` = 2 (Binormal) ← **关键值**
- **data_type**: `34 03 00 00` = 820 (Float3)
- **buffer_index**: `00 00 00 00` = 0 (buffer0)
- **buffer_offset**: `18 00 00 00` = 24字节 (6个float × 4字节)
- **subindex**: `01 00 00 00` = 1

### 属性4: Binormal1 (第二个副法线)
- **usage**: `03 00 00 00` = 3 (Tangent) ← **关键值**
- **data_type**: `34 03 00 00` = 820 (Float3)
- **buffer_index**: `00 00 00 00` = 0 (buffer0)
- **buffer_offset**: `24 00 00 00` = 36字节
- **subindex**: `02 00 00 00` = 2

### 属性5: Tangent0 (切线)
- **usage**: `03 00 00 00` = 3 (Tangent)
- **data_type**: `34 03 00 00` = 820 (Float3)
- **buffer_index**: `00 00 00 00` = 0 (buffer0)
- **buffer_offset**: `30 00 00 00` = 48字节
- **subindex**: `03 00 00 00` = 3

### 属性6: TextureCoordinate0 (第一组UV)
- **usage**: `04 00 00 00` = 4 (TextureCoordinate)
- **data_type**: `37 04 00 00` = 1079 (Float2)
- **buffer_index**: `01 00 00 00` = 1 (buffer1)
- **buffer_offset**: `00 00 00 00` = 0字节
- **subindex**: `00 00 00 00` = 0

### 属性7: TextureCoordinate1 (第二组UV)
- **usage**: `04 00 00 00` = 4 (TextureCoordinate)
- **data_type**: `34 04 00 00` = 1076 (Float4)
- **buffer_index**: `01 00 00 00` = 1 (buffer1)
- **buffer_offset**: `08 00 00 00` = 8字节 (2个float × 4字节)
- **subindex**: `01 00 00 00` = 1

### 属性8: ColorSet0 (颜色集)
- **usage**: `05 00 00 00` = 5 (ColorSet)
- **data_type**: `37 04 00 00` = 1079 (Float2)
- **buffer_index**: `01 00 00 00` = 1 (buffer1)
- **buffer_offset**: `20 00 00 00` = 32字节
- **subindex**: `00 00 00 00` = 0

## 生成代码路径

### 1. 主要生成函数
文件：`ssbh_data/src/mesh_data/mesh_attributes.rs`

```rust
pub fn create_attributes_v8(data: &MeshObjectData, is_vs2: bool) -> MeshAttributes<AttributeV8> {
    let buffer0_data = get_positions_v8(&data.positions, AttributeUsageV8::Position)
        .chain(get_vectors_v8(&data.normals, AttributeUsageV8::Normal))
        .chain(get_vectors_v8(&data.tangents, AttributeUsageV8::Tangent))
        .collect_vec();

    let buffer1_data = get_vectors_v8(&data.texture_coordinates, AttributeUsageV8::TextureCoordinate)
        .chain(get_colors_v8(&data.color_sets, AttributeUsageV8::ColorSet))
        .collect_vec();
    // ...
}
```

### 2. 属性创建函数
```rust
fn create_attribute_v8(
    _name: &str,
    subindex: usize,
    buffer_index: u32,
    usage: AttributeUsageV8,     // ← usage值在这里设置
    data_type: AttributeDataTypeV8,
    buffer_offset: usize,
    _is_vs2: bool,
) -> AttributeV8 {
    AttributeV8 {
        usage,
        data_type,
        buffer_index,
        buffer_offset: buffer_offset as u32,
        subindex: subindex as u32,
    }
}
```

### 3. MeshData转换
文件：`ssbh_data/src/mesh_data.rs`

```rust
impl TryFrom<&MeshData> for Mesh {
    fn try_from(data: &MeshData) -> Result<Self, Self::Error> {
        create_mesh(data)
    }
}

fn create_mesh(data: &MeshData) -> Result<Mesh, error::Error> {
    match (data.major_version, data.minor_version) {
        (1, 8) => Ok(Mesh::V8(create_mesh_inner(
            &all_positions,
            create_mesh_objects(&data.objects, |obj| create_attributes_v8(obj, is_vs2))?,
            data,
        )?)),
        // ...
    }
}
```

## 重要发现

### `03 00 00 00` 和 `02 00 00 00` 的含义

- **`03 00 00 00`**: 表示AttributeUsageV8::Tangent (切线属性)
- **`02 00 00 00`**: 表示AttributeUsageV8::Binormal (副法线属性)

这些值是在V8格式的属性数组中，表示不同类型的顶点属性用途。

### 数据布局特点

1. **Buffer分组**: 
   - Buffer0: Position, Normal, Binormal, Tangent (几何属性)
   - Buffer1: TextureCoordinate, ColorSet (材质属性)

2. **偏移计算**: 
   - 每个属性的buffer_offset是前面所有属性大小的累积
   - Float3 = 12字节, Float2 = 8字节, Float4 = 16字节

3. **子索引**: 
   - 同类型属性使用不同的subindex来区分
   - 如TextureCoordinate0(subindex=0), TextureCoordinate1(subindex=1)

## 调试建议

如果需要修改这些值的生成，主要关注以下文件：

1. `ssbh_lib/src/formats/mesh.rs` - 枚举定义
2. `ssbh_data/src/mesh_data/mesh_attributes.rs` - 属性创建逻辑
3. `ssbh_data/src/mesh_data.rs` - 整体转换流程

## 版本信息

- 分析基于: SSBH v1.8格式
- 库版本: ssbh_lib, ssbh_data
- VS2模式: 是（不包含属性名字符串以节省空间）
