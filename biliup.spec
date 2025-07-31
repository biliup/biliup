# -*- mode: python ; coding: utf-8 -*-
from PyInstaller.utils.hooks import collect_all, copy_metadata

datas = [('biliup/database/migration/', 'biliup/database/migration/'), ('biliup/web/public/', 'biliup/web/public/'), ('biliup/Danmaku/douyin_util/', 'biliup/Danmaku/douyin_util/')]
binaries = []
hiddenimports = []
tmp_ret = collect_all('biliup.plugins')
datas += tmp_ret[0]; binaries += tmp_ret[1]; hiddenimports += tmp_ret[2]
# datas += copy_metadata('biliup')

a = Analysis(
    ['biliup\\__main__.py'],
    pathex=[],
    binaries=binaries,
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
    optimize=0,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    [], # a.binaries 从这里移除
    [], # a.datas 从这里移除
    [],
    name='biliup',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=['public\\logo.png'],
)

# One-Folder Mode
coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name='bbup' # 这是最终生成的文件夹的名称
)