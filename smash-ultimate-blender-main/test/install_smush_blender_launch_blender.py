#!python3.11
"""
Tested on windows 10
Please edit the `blender_bin` variable for different blender versions.
make sure to pip install the right version as well, if possible.
also change the comment at the top of the file if windows is launching the incorrect python version.
Sometimes the newest blender version's bpy module wont be available yet for pip installation, causing issues with the install portion of this script.
Example installing bpy module for specific python version: py -3.10 -m pip install bpy
"""

from zipfile import ZipFile, ZIP_DEFLATED
from pathlib import Path
from platform import python_version

def zip_dir(dir_to_zip: Path, zip_file_path: Path, file_names_to_ignore: set[Path]) -> None:
    paths_to_zip: set[Path] = set()
    for path in dir_to_zip.iterdir():
        if path.name in file_names_to_ignore:
            continue
        if path.is_dir():
            paths_to_zip.update(path.rglob("*"))
        else:
            paths_to_zip.add(path)

    with ZipFile(zip_file_path, 'w') as zip:
        for path in paths_to_zip:
            zip.write(path, arcname=Path("smash-ultimate-blender") / path.relative_to(dir_to_zip), compress_type=ZIP_DEFLATED, compresslevel=1)

def install_zipped_plugin(zipped_plugin: Path, blender_bin: Path) -> None:
    import bpy
    import subprocess

    bpy.app.binary_path = blender_bin
    bpy.ops.preferences.addon_install(filepath=str(zipped_plugin))
    bpy.ops.preferences.addon_enable(module="smash-ultimate-blender")
    bpy.ops.wm.save_userpref()
    subprocess.run([blender_bin])

def main():
    from ..__init__ import bl_info
    version = bl_info['version']
    temp_zip_path = Path(__file__).parent / Path(f'smash-ultimate-blender_{version[0]}_{version[1]}_{version[2]}.zip')
    top_level_dir = Path(__file__).parent.parent
    ignore = {".git", "test", ".gitignore", "README.md", "__pycache__"}
    blender_bin = r"C:\Program Files\Blender Foundation\Blender 4.2\blender.exe"
    print(blender_bin)
    print(python_version())
    zip_dir(top_level_dir, temp_zip_path, ignore)
    try:
        import bpy
    except Exception as e:
        print("Unable to import bpy, won't attempt automatic install")
    else:
        install_zipped_plugin(temp_zip_path, blender_bin)
    
    #temp_zip_path.unlink()
    
if __name__ == '__main__' and __package__ is None:
    
    if __package__ is None:
        import sys
        import importlib
        repos_path = Path(__file__).parent.parent.parent
        sys.path.append(str(repos_path))
        smash_ultimate_blender = importlib.import_module("smash-ultimate-blender")
        __package__ = "smash-ultimate-blender.test"
    
    main()