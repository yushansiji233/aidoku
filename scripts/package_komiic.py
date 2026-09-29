import json
import shutil
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED

root = Path(__file__).resolve().parents[1]
source = root / 'komiic-cc'
wasm = source / 'target/wasm32-unknown-unknown/release/komiic_cc.wasm'
if not wasm.is_file():
    raise SystemExit(f'Missing compiled source: {wasm}')

sources = root / 'sources'
icons = root / 'icons'
sources.mkdir(exist_ok=True)
icons.mkdir(exist_ok=True)
archive = sources / 'zh.komiiccc-v1.aix'
with ZipFile(archive, 'w', ZIP_DEFLATED) as output:
    for path, name in (
        (wasm, 'main.wasm'),
        (source / 'res/source.json', 'source.json'),
        (source / 'res/settings.json', 'settings.json'),
        (source / 'res/filters.json', 'filters.json'),
        (source / 'res/icon.png', 'icon.png'),
    ):
        output.write(path, f'Payload/{name}')
shutil.copyfile(source / 'res/icon.png', icons / 'zh.komiiccc-v1.png')

manifest_path = root / 'index.min.json'
manifest = json.loads(manifest_path.read_text(encoding='utf-8-sig'))
entry = {
    'id': 'zh.komiiccc',
    'name': 'Komiic.cc',
    'version': 1,
    'iconURL': 'icons/zh.komiiccc-v1.png',
    'downloadURL': 'sources/zh.komiiccc-v1.aix',
    'languages': ['zh'],
    'contentRating': 0,
    'baseURL': 'https://komiic.cc',
}
manifest['sources'] = [s for s in manifest['sources'] if s['id'] != entry['id']]
manifest['sources'].insert(0, entry)
manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, separators=(',', ':')), encoding='utf-8')
print(f'Packaged {archive}; list has {len(manifest["sources"])} sources')
