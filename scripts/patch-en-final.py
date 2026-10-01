# -*- coding: utf-8 -*-
"""第三轮补漏:全量清除界面残留英文。"""

# (类型, 文件通配, 英文, 中文)
TEXTS = [
    '(optional)', '(可选)',
    'Cancel', '取消',
    'Close to Tray', '关闭到托盘',
    'Confirm', '确认',
    'Delete All', '全部删除',
    'Delete Selected', '删除选中',
    'Download Rate (KB/s)', '下载速率 (KB/s)',
    'Upload Rate (KB/s)', '上传速率 (KB/s)',
    'Download complete — now seeding', '下载完成 — 正在做种',
    'Downloaded ↓', '已下载 ↓',
    'Uploaded ↑', '已上传 ↑',
    'Imported instances will sync their announce port from Gluetun.', '导入实例的 announce 端口将与 Gluetun 同步。',
    'Keep Running', '保持运行',
    'Quit', '退出',
    'Refresh Interval (sec)', '刷新间隔 (秒)',
    'Scrape Interval (sec)', 'Scrape 间隔 (秒)',
    'Refresh Interval', '刷新间隔',
    'Scrape Interval', 'Scrape 间隔',
    'Select a folder from your computer', '从电脑选择文件夹',
    'Start Now', '立即开始',
    'Stop Now', '立即停止',
    'Stop & Idle Conditions', '停止与空闲条件',
    'Upload Rate', '上传速率',
    'Download Rate', '下载速率',
    'Why do I need this?', '为什么需要这个?',
    '↑ Upload Range', '↑ 上传区间',
    '↓ Download Range', '↓ 下载区间',
    '↑ Upload', '↑ 上传',
    '↓ Download', '↓ 下载',
]
# 顺序敏感:长的先替换
ATTRS = [
    ('placeholder', '/path/to/directory', '目录路径'),
    ('placeholder', '/path/to/torrents', '种子目录路径'),
    ('placeholder', '/path/to/watch', '监视目录路径'),
    ('placeholder', 'tag1, tag2, tag3', '标签1,标签2,标签3'),
    ('title', 'Stop', '停止'),
]

# JS 里的用户可见字符串(精确整串替换)
JS_STRINGS = [
    ('lib/instanceStore.js', "'Torrent file not found - please select again'", "'未找到种子文件,请重新选择'"),
    ('lib/instanceStore.js', "'Please re-upload your torrent file'", "'请重新上传种子文件'"),
    ('lib/instanceStore.js', "'restored from desktop state'", "'已从桌面状态恢复'"),
    ('lib/status.js', "'Stop condition met'", "'停止条件已满足'"),
    ('lib/customPreset.js', "'Instance not found.'", "'未找到实例。'"),
    ('lib/customPreset.js', "'Please enter a preset name.'", "'请输入预设名称。'"),
    ('lib/customPreset.js', "'Invalid preset object'", "'无效的预设对象'"),
    ('lib/api.js', "'Failed to load torrent'", "'加载种子失败'"),
    ('lib/api.js', "'Authentication required'", "'需要身份验证'"),
    ('lib/api.js', "'All IP lookup services failed'", "'全部 IP 查询服务失败'"),
    ('lib/api.js', "'No valid file paths available. Use folder import or file dialog.'", "'没有可用的文件路径,请使用文件夹导入或文件选择。'"),
]

# gridFilterOptions:状态筛选标签
GRID_LABELS = [
    ("label: 'All States'", "label: '全部状态'"),
    ("label: 'Starting'", "label: '启动中'"),
    ("label: 'Stopping'", "label: '停止中'"),
    ("label: 'Stopped'", "label: '已停止'"),
    ("label: 'All Tags'", "label: '全部标签'"),
    ("'Untagged'", "'无标签'"),
]

# themeStore:主题显示名与描述
THEME = [
    ("label: 'Default'", "label: '默认'"),
    ("label: 'Follow system preference'", "label: '跟随系统'"),
    ("description: 'Default light theme'", "description: '默认浅色主题'"),
    ("description: 'Default dark theme'", "description: '默认深色主题'"),
    ("description: 'Light with warm tones'", "description: '暖色调浅色'"),
    ("description: 'Muted dark theme'", "description: '低饱和深色'"),
    ("description: 'Medium contrast dark'", "description: '中等对比深色'"),
    ("description: 'The original dark'", "description: '经典深色'"),
]

# watchTree:根节点标签
WATCHTREE = [
    ("'Root'", "'根目录'"),
]

import io
import pathlib

changed = set()

def patch(path, pairs, exact=True):
    p = pathlib.Path(path)
    if not p.exists():
        print('MISSING', path)
        return
    s = io.open(p, encoding='utf-8').read()
    orig = s
    for old, new in pairs:
        if old in s:
            s = s.replace(old, new)
    if s != orig:
        io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
        changed.add(str(p))
        print('OK', path)

# 1) Svelte 文本节点:精确 >text< 替换
for f in sorted(pathlib.Path('ui/src').rglob('*.svelte')):
    s = io.open(f, encoding='utf-8').read()
    orig = s
    for i in range(0, len(TEXTS), 2):
        en, zh = TEXTS[i], TEXTS[i + 1]
        # 文本节点形式(允许首尾空白)
        s = s.replace(f'>{en}<', f'>{zh}<')
    for attr, en, zh in ATTRS:
        s = s.replace(f'{attr}="{en}"', f'{attr}="{zh}"')
    if s != orig:
        io.open(f, 'w', encoding='utf-8', newline='\n').write(s)
        changed.add(str(f))
        print('OK', f)

# 2) JS 精确串
for path, old, new in JS_STRINGS:
    patch(path, [(old, new)])

# 3) 网格筛选/主题/目录树
patch('ui/src/lib/gridFilterOptions.js', GRID_LABELS)
patch('ui/src/lib/gridFilters.js', WATCHTREE + GRID_LABELS[-2:])  # Untagged 在两个文件
patch('ui/src/lib/themeStore.svelte.js', THEME)
patch('ui/src/lib/watchTree.js', WATCHTREE)

print(f'\n共修改 {len(changed)} 个文件')
