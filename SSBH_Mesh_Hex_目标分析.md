# SSBH Mesh数据Hex目标顺序分析报告

## 概述

本文档分析了需要调整的SSBH v1.8格式中AttributeV8数组的目标hex数据结构，并与现有实现进行对比，提供具体的调整方案。

## 目标Hex数据分析

### 原始目标Hex数据
```
00 00 00 00 34 03 00 00 00 00 00 00 00 00 00 00 00 00 00 00 
01 00 00 00 34 03 00 00 00 00 00 00 0C 00 00 00 00 00 00 00 
02 00 00 00 34 03 00 00 00 00 00 00 18 00 00 00 00 00 00 00 
03 00 00 00 34 03 00 00 00 00 00 00 24 00 00 00 00 00 00 00 
02 00 00 00 34 03 00 00 00 00 00 00 30 00 00 00 00 00 00 00 
03 00 00 00 34 03 00 00 00 00 00 00 3C 00 00 00 00 00 00 00 
04 00 00 00 37 04 00 00 01 00 00 00 00 00 00 00 00 00 00 00 
05 00 00 00 37 04 00 00 01 00 00 00 08 00 00 00 00 00 00 00 
08 00 00 00 34 04 00 00 01 00 00 00 10 00 00 00 00 00 00 00
```

总计：9个属性 × 20字节 = 180字节

## 与现有数据对比

### 现有数据结构（原文档）
```
00 00 00 00 34 03 00 00 00 00 00 00 00 00 00 00 00 00 00 00  // Position0
01 00 00 00 34 03 00 00 00 00 00 00 0C 00 00 00 00 00 00 00  // Normal0
03 00 00 00 34 03 00 00 00 00 00 00 18 00 00 00 01 00 00 00  // Binormal0 (错误的usage)
03 00 00 00 34 03 00 00 00 00 00 00 24 00 00 00 02 00 00 00  // Binormal1 (错误的usage)
03 00 00 00 34 03 00 00 00 00 00 00 30 00 00 00 03 00 00 00  // Tangent0
04 00 00 00 37 04 00 00 01 00 00 00 00 00 00 00 00 00 00 00  // TextureCoordinate0
04 00 00 00 34 04 00 00 01 00 00 00 08 00 00 00 01 00 00 00  // TextureCoordinate1
05 00 00 00 37 04 00 00 01 00 00 00 20 00 00 00 00 00 00 00  // ColorSet0
```

### 目标数据结构（期望的顺序）
```
00 00 00 00 34 03 00 00 00 00 00 00 00 00 00 00 00 00 00 00  // Position0
01 00 00 00 34 03 00 00 00 00 00 00 0C 00 00 00 00 00 00 00  // Normal0
02 00 00 00 34 03 00 00 00 00 00 00 18 00 00 00 00 00 00 00  // Binormal0 (正确的usage=2)
03 00 00 00 34 03 00 00 00 00 00 00 24 00 00 00 00 00 00 00  // Tangent0 (正确的usage=3)
02 00 00 00 34 03 00 00 00 00 00 00 30 00 00 00 00 00 00 00  // Binormal1 (正确的usage=2)
03 00 00 00 34 03 00 00 00 00 00 00 3C 00 00 00 00 00 00 00  // Tangent1 (正确的usage=3)
04 00 00 00 37 04 00 00 01 00 00 00 00 00 00 00 00 00 00 00  // TextureCoordinate0
05 00 00 00 37 04 00 00 01 00 00 00 08 00 00 00 00 00 00 00  // ColorSet0
08 00 00 00 34 04 00 00 01 00 00 00 10 00 00 00 00 00 00 00  // HalfFloat2 (新增)
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
- **buffer_offset**: `0C 00 00 00` = 12字节
- **subindex**: `00 00 00 00` = 0

### 属性3: Binormal0 (副法线) ✅ 修正
- **usage**: `02 00 00 00` = 2 (Binormal) ← **修正为正确的usage值**
- **data_type**: `34 03 00 00` = 820 (Float3)
- **buffer_index**: `00 00 00 00` = 0 (buffer0)
- **buffer_offset**: `18 00 00 00` = 24字节
- **subindex**: `00 00 00 00` = 0 ← **修正为0**

### 属性4: Tangent0 (切线) ✅ 修正
- **usage**: `03 00 00 00` = 3 (Tangent) ← **修正为正确的usage值**
- **data_type**: `34 03 00 00` = 820 (Float3)
- **buffer_index**: `00 00 00 00` = 0 (buffer0)
- **buffer_offset**: `24 00 00 00` = 36字节
- **subindex**: `00 00 00 00` = 0 ← **修正为0**

### 属性5: Binormal1 (第二副法线) ✅ 修正
- **usage**: `02 00 00 00` = 2 (Binormal) ← **修正为正确的usage值**
- **data_type**: `34 03 00 00` = 820 (Float3)
- **buffer_index**: `00 00 00 00` = 0 (buffer0)
- **buffer_offset**: `30 00 00 00` = 48字节
- **subindex**: `00 00 00 00` = 0 ← **修正为0**

### 属性6: Tangent1 (第二切线) ✅ 修正
- **usage**: `03 00 00 00` = 3 (Tangent) ← **修正为正确的usage值**
- **data_type**: `34 03 00 00` = 820 (Float3)
- **buffer_index**: `00 00 00 00` = 0 (buffer0)
- **buffer_offset**: `3C 00 00 00` = 60字节
- **subindex**: `00 00 00 00` = 0 ← **修正为0**

### 属性7: TextureCoordinate0 (第一组UV)
- **usage**: `04 00 00 00` = 4 (TextureCoordinate)
- **data_type**: `37 04 00 00` = 1079 (Float2)
- **buffer_index**: `01 00 00 00` = 1 (buffer1)
- **buffer_offset**: `00 00 00 00` = 0字节
- **subindex**: `00 00 00 00` = 0

### 属性8: ColorSet0 (颜色集)
- **usage**: `05 00 00 00` = 5 (ColorSet)
- **data_type**: `37 04 00 00` = 1079 (Float2)
- **buffer_index**: `01 00 00 00` = 1 (buffer1)
- **buffer_offset**: `08 00 00 00` = 8字节
- **subindex**: `00 00 00 00` = 0

### 属性9: HalfFloat2 (新增) ✅ 新增
- **usage**: `08 00 00 00` = 8 (HalfFloat2) ← **新增的属性类型**
- **data_type**: `34 04 00 00` = 1076 (Float4)
- **buffer_index**: `01 00 00 00` = 1 (buffer1)
- **buffer_offset**: `10 00 00 00` = 16字节
- **subindex**: `00 00 00 00` = 0

## 主要变化分析

### 1. Usage值修正
原有代码中存在错误：
- **错误**: Binormal属性被设置为Tangent usage (3)
- **修正**: Binormal属性应使用正确的Binormal usage (2)

### 2. Subindex修正
原有代码中subindex递增：
- **错误**: subindex从1开始递增 (1, 2, 3...)
- **修正**: 所有Buffer0中的属性subindex都应为0

### 3. 属性顺序调整
理想顺序应该是：
1. Position0 (usage=0)
2. Normal0 (usage=1)  
3. Binormal0 (usage=2, subindex=0)
4. Tangent0 (usage=3, subindex=0)
5. Binormal1 (usage=2, subindex=0)
6. Tangent1 (usage=3, subindex=0)
7. TextureCoordinate0 (usage=4)
8. ColorSet0 (usage=5)
9. HalfFloat2 (usage=8) - 新增

### 4. Buffer偏移重新计算
由于增加了额外的Tangent1和HalfFloat2属性：
- Buffer0总大小: 6个Float3 = 72字节 (0x48)
- Buffer1偏移需要重新计算

## 代码修改建议

### 修改`ssbh_data/src/mesh_data/mesh_attributes.rs`中的`create_attributes_v8`函数

```rust
pub fn create_attributes_v8(data: &MeshObjectData, is_vs2: bool) -> MeshAttributes<AttributeV8> {
    // 修正Buffer0的属性创建顺序
    let buffer0_data = get_positions_v8(&data.positions, AttributeUsageV8::Position)
        .chain(get_vectors_v8(&data.normals, AttributeUsageV8::Normal))
        .chain(get_vectors_v8(&data.binormals, AttributeUsageV8::Binormal))  // 修正: 先binormals
        .chain(get_vectors_v8(&data.tangents, AttributeUsageV8::Tangent))   // 修正: 后tangents
        .collect_vec();

    // 修正Buffer1的属性创建顺序，添加HalfFloat2
    let buffer1_data = get_vectors_v8(&data.texture_coordinates, AttributeUsageV8::TextureCoordinate)
        .chain(get_colors_v8(&data.color_sets, AttributeUsageV8::ColorSet))
        .chain(get_half_float2_v8(&data.half_float2_data, AttributeUsageV8::HalfFloat2))  // 新增
        .collect_vec();

    // 修正subindex逻辑，确保同类型属性正确索引
    // ...
}
```

### 修改subindex生成逻辑

确保：
- 同一种usage类型的多个属性使用正确的subindex
- Binormal和Tangent属性的subindex都从0开始，而不是全局递增

### 添加HalfFloat2支持

需要在MeshObjectData结构中添加对HalfFloat2数据的支持，并实现相应的生成函数。

## 验证步骤

1. **修改属性生成逻辑**：确保usage值正确分配
2. **修改subindex逻辑**：确保subindex正确计算
3. **添加新属性类型**：实现HalfFloat2属性支持
4. **测试生成结果**：验证生成的hex数据与目标一致
5. **检查buffer偏移**：确保所有偏移值正确计算

## 关键文件位置

1. `ssbh_lib/src/formats/mesh.rs` - AttributeUsageV8和AttributeDataTypeV8枚举
2. `ssbh_data/src/mesh_data/mesh_attributes.rs` - 属性创建和排序逻辑
3. `ssbh_data/src/mesh_data.rs` - MeshObjectData结构定义
4. `ssbh_data/src/mesh_data.rs` - 整体转换流程

## ssbh_data_dae.rs文件更新分析

### 当前问题分析

通过检查`src/convert/ssbh_data_dae.rs`文件，发现以下问题与目标hex顺序不匹配：

#### 1. 属性创建顺序问题
```rust
// 当前实现 (第145-174行)
binormals: vec![
    AttributeData { /* Binormal0 */ },
    AttributeData { /* Binormal1 */ },
],
tangents: vec![
    AttributeData { /* Tangent0 */ },
    AttributeData { /* Tangent1 */ },
    AttributeData { /* Tangent2 */ },  // ⚠️ 多余的tangent
    AttributeData { /* Tangent3 */ },  // ⚠️ 多余的tangent
],
```

**问题**: 
- 创建了4个tangent属性，但目标hex只需要2个
- 没有考虑ssbh_data库内部的属性交替排列逻辑

#### 2. HalfFloat2属性处理问题
```rust
// 当前实现 (第176-185行)
texture_coordinates: vec![
    AttributeData { /* TextureCoordinate0 */ },
    AttributeData { 
        name: "HalfFloat2_0".to_string(),  // ⚠️ 错误的属性分类
        data: VectorData::Vector4(generate_texture_coordinates_halffloat2_data(vertex_count)),
    },
],
```

**问题**: 
- HalfFloat2被错误地放在texture_coordinates数组中
- 应该是独立的属性类型，usage=8

#### 3. 数据生成逻辑问题
```rust
// 第414-416行
fn generate_texture_coordinates_halffloat2_data(vertex_count: usize) -> Vec<[f32; 4]> {
    vec![[1.0, 1.0, 1.0, 1.0]; vertex_count]  // ⚠️ 错误的数据类型
}
```

**问题**: 
- HalfFloat2应该生成Float4数据但usage应为8
- 默认值可能不正确

### 需要的更新

#### 1. 修正属性创建逻辑
```rust
// 应该修改为 (第144-174行)
// Binormal0 - required
binormals: vec![AttributeData {
    name: "".to_string(),
    data: VectorData::Vector3(binormals.clone()),
}],
// Tangent0 - required  
tangents: vec![AttributeData {
    name: "".to_string(),
    data: VectorData::Vector3(tangents.clone()),
}],
```

#### 2. 添加独立的HalfFloat2支持
```rust
// 新增字段到MeshObjectData结构中
half_float2_data: vec![AttributeData {
    name: "".to_string(),
    data: VectorData::Vector4(generate_half_float2_data(vertex_count)),
}],
```

#### 3. 更新数据生成函数
```rust
fn generate_half_float2_data(vertex_count: usize) -> Vec<[f32; 4]> {
    // 根据目标hex数据生成正确的默认值
    vec![[0.0, 0.0, 0.0, 0.0]; vertex_count] // 或根据分析的正确值
}
```

### 为什么需要更新

#### 1. **属性数量不匹配**
- 当前: Position(1) + Normal(1) + Binormal(2) + Tangent(4) + UV(2) + Color(1) = 11个
- 目标: Position(1) + Normal(1) + Binormal(2) + Tangent(2) + UV(1) + Color(1) + HalfFloat2(1) = 9个

#### 2. **属性类型错误**
- HalfFloat2被错误地归类为纹理坐标
- 应该是独立的属性类型，usage=8

#### 3. **依赖ssbh_data库的修改**
- `ssbh_data_dae.rs`文件依赖ssbh_data库的内部逻辑
- 如果ssbh_data库修改了属性排序和subindex逻辑，此文件必须相应调整

#### 4. **数据一致性**
- 确保生成的MeshObjectData结构能正确转换为目标hex格式
- 避免在转换过程中产生不必要的属性

### 更新优先级

1. **高优先级**: 修正属性数量和类型分类
2. **中优先级**: 调整数据生成逻辑以匹配预期值
3. **低优先级**: 优化代码注释和日志输出

### 验证方法

1. 修改文件后测试DAE转换
2. 检查生成的hex数据是否与目标匹配
3. 确保所有9个属性都正确生成
4. 验证buffer偏移和subindex值

## 总结

目标hex数据要求的主要调整：
1. **修正usage值**: Binormal使用2，Tangent使用3
2. **修正subindex**: Buffer0中所有属性subindex为0
3. **调整属性顺序**: Position → Normal → Binormal → Tangent → Binormal → Tangent → UV → Color → HalfFloat2
4. **新增属性类型**: 添加HalfFloat2 (usage=8) 支持
5. **重新计算偏移**: 根据新的属性顺序和大小重新计算buffer偏移

**ssbh_data_dae.rs文件必须同步更新**：
- 减少tangent属性数量从4个到2个
- 将HalfFloat2从texture_coordinates中分离出来作为独立属性
- 确保生成的MeshObjectData结构与ssbh_data库的新逻辑兼容
- 修正数据生成函数以产生正确的默认值

这些修改需要在ssbh_data库的属性生成逻辑中实现，同时ssbh_data_dae.rs文件也需要相应调整，确保生成的二进制数据与目标hex完全匹配。
