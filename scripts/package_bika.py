import json
import re
import shutil
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

root = Path(__file__).resolve().parents[1]
source = root / 'bika-web'
info = json.loads((source / 'res/source.json').read_text(encoding='utf-8-sig'))['info']
version = info['version']
wasm = source / 'target/wasm32-unknown-unknown/release/bika_web.wasm'
if not wasm.is_file():
    raise SystemExit(f'Missing compiled source: {wasm}')

sources = root / 'sources'
icons = root / 'icons'
sources.mkdir(exist_ok=True)
icons.mkdir(exist_ok=True)
archive = sources / f'zh.manhuabika-v{version}.aix'
with ZipFile(archive, 'w', ZIP_DEFLATED) as output:
    for path, name in (
        (wasm, 'main.wasm'),
        (source / 'res/source.json', 'source.json'),
        (source / 'res/settings.json', 'settings.json'),
        (source / 'res/filters.json', 'filters.json'),
        (source / 'res/icon.png', 'icon.png'),
    ):
        output.write(path, f'Payload/{name}')
shutil.copyfile(source / 'res/icon.png', icons / f'zh.manhuabika-v{version}.png')

manifest_path = root / 'index.min.json'
manifest = json.loads(manifest_path.read_text(encoding='utf-8-sig'))
entry = {
    'id': 'zh.manhuabika',
    'name': '哔咔漫画',
    'version': version,
    'iconURL': f'icons/zh.manhuabika-v{version}.png',
    'downloadURL': f'sources/zh.manhuabika-v{version}.aix',
    'languages': ['zh'],
    'contentRating': 2,
    'baseURL': 'https://manhuabika.com',
}
manifest['sources'] = [s for s in manifest['sources'] if s['id'] != entry['id']]
manifest['sources'].append(entry)
manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, separators=(',', ':')), encoding='utf-8')
# Remove only obsolete versioned artifacts for this source after packaging succeeds.
for directory, suffix, current in (
    (sources, 'aix', archive.name),
    (icons, 'png', f'zh.manhuabika-v{version}.png'),
):
    for path in directory.iterdir():
        if (path.name != current
                and re.fullmatch(r'zh\.manhuabika-v\d+\.' + suffix, path.name)
                and path.is_file() and not path.is_symlink()):
            path.unlink()
            print(f'Removed obsolete artifact: {path.name}')
print(f'Packaged {archive}; list has {len(manifest["sources"])} sources')
