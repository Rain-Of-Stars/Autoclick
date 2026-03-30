import os
import re
import glob

def refactor_header():
    for root, _, files in os.walk('src/pages'):
        for file in files:
            if not file.endswith('.tsx'): continue
            path = os.path.join(root, file)
            with open(path, 'r', encoding='utf8') as f:
                content = f.read()

            # Swap <p className="desk-eyebrow">...</p> and <h1 className="desk-title">...</h1> or similar combinations
            pattern = re.compile(
                r'<p className="desk-eyebrow">([^<]+)</p>\s*<([h\d]+) className="desk-title">([^<]+)</\2>', 
                re.MULTILINE
            )
            # wait, live preview has:
            # <p className="desk-eyebrow">Live Preview</p>
            # <div className="mt-2 flex flex-wrap items-center gap-3">
            #   <h1 className="desk-title">实时预览</h1>
            pass

refactor_header()
