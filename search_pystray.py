import pathlib
root=pathlib.Path('D:/Github_project/Autoclick')
for p in root.rglob('*'):
    if not p.is_file():
        continue
    try:
        text = p.read_text('utf-8')
    except UnicodeDecodeError:
        continue
    if 'pystray' in text:
        print(p)
