import pathlib
root = pathlib.Path('D:/Github_project/Autoclick')
keywords = ['WM_DELETE_WINDOW', 'icon.stop', 'mainloop(']
skipped = {'search_strings.py'}
for p in root.rglob('*.py'):
    if p.name in skipped:
        continue
    try:
        text = p.read_text(encoding='utf-8')
    except Exception:
        continue
    lines = text.splitlines()
    for idx, line in enumerate(lines, 1):
        if any(kw in line for kw in keywords):
            print(f'{p}:{idx}:{line.strip()}')
