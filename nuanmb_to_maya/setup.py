"""
Setup script for nuanmb-to-maya package
"""

from setuptools import setup, find_packages
from pathlib import Path

# Read README for long description
readme_file = Path(__file__).parent / "README.md"
long_description = ""
if readme_file.exists():
    long_description = readme_file.read_text(encoding='utf-8')

setup(
    name='nuanmb-to-maya',
    version='0.1.0',
    description='Convert NUANMB animation files from Super Smash Bros. Ultimate to Maya .anim format',
    long_description=long_description,
    long_description_content_type='text/markdown',
    author='SSBH Tools Community',
    url='https://github.com/ScanMountGoat/ssbh_editor',
    packages=find_packages(),
    install_requires=[
        'numpy>=1.21.0',
    ],
    python_requires='>=3.8',
    entry_points={
        'console_scripts': [
            'nuanmb2maya=main:main',
        ],
    },
    classifiers=[
        'Development Status :: 3 - Alpha',
        'Intended Audience :: Developers',
        'Topic :: Multimedia :: Graphics :: 3D Modeling',
        'Programming Language :: Python :: 3',
        'Programming Language :: Python :: 3.8',
        'Programming Language :: Python :: 3.9',
        'Programming Language :: Python :: 3.10',
        'Programming Language :: Python :: 3.11',
    ],
    keywords='smash-bros animation maya 3d-modeling nuanmb',
)

