import bpy

from bpy.types import Panel

from typing import TYPE_CHECKING
if TYPE_CHECKING:
    from ..anim.anim_data import SUB_PG_sub_anim_data

class SUB_PT_misc(Panel):
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = 'Ultimate'
    bl_label = 'Misc.'
    bl_options = {'DEFAULT_CLOSED'}

    @classmethod
    def poll(cls, context):
        modes = ['POSE', 'OBJECT']
        return context.mode in modes

    def draw(self, context):
        ssp: SUB_PG_sub_anim_data = context.scene.sub_scene_properties

        layout = self.layout
        layout.use_property_split = False
        
        row = layout.row(align=True)
        row.operator("sub.eye_material_custom_vector_31_modal")

        # Add renaming tools operators (only available in Object mode)
        layout.separator()
        box = layout.box()
        box.label(text="Renaming Tools")
        
        # Rename Materials to Mesh button
        row = box.row(align=True)
        if context.mode == 'OBJECT':
            row.operator("sub.rename_materials_to_mesh", text="Rename Materials to Mesh")
        else:
            row.enabled = False
            row.operator("sub.rename_materials_to_mesh", text="Rename Materials to Mesh (Object Mode Only)")
        
        # Rename Textures to Material button
        row = box.row(align=True)
        if context.mode == 'OBJECT':
            row.operator("sub.rename_textures_to_material", text="Rename Textures to Material")
        else:
            row.enabled = False
            row.operator("sub.rename_textures_to_material", text="Rename Textures to Material (Object Mode Only)")

    