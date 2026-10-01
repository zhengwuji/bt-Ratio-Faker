# -*- coding: utf-8 -*-
"""读取 tauri.conf.json 的版本号;--bump 时把 -vN 加一。"""
import io
import json
import re
import sys

CONF = 'rustatio-desktop/tauri.conf.json'


def main():
    bump = '--bump' in sys.argv
    conf = json.load(io.open(CONF, encoding='utf-8'))
    version = conf.get('version', '0.0.0')
    m = re.search(r'-v(\d+)$', version)
    if bump:
        n = (int(m.group(1)) + 1) if m else 1
        version = f'{version.split("-v")[0]}-v{n}'
        conf['version'] = version
        for w in conf.get('app', {}).get('windows', []):
            w['title'] = re.sub(r' v\d+', f' v{n}', w.get('title', 'Rustatio'))
            if ' v' not in w['title']:
                w['title'] = f"Rustatio v{n} - BitTorrent 比率管理工具"
        io.open(CONF, 'w', encoding='utf-8', newline='\n').write(
            json.dumps(conf, indent=2, ensure_ascii=False) + '\n')
    print(version)


if __name__ == '__main__':
    main()
