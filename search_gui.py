import pathlib
root = pathlib.Path('D:/Github_project/Autoclick')
keywords = ['tkinter', 'pystray', 'MainLoop', 'mainloop']
for p in root.rglob('*.py'):
    if p.name in {'search_strings.py','search_gui.py'}:
        continue
    text = p.read_text(encoding='utf-8', errors='ignore')
    lower = text.lower()
    if any(kw.lower() in lower for kw in keywords):
        print(p)
