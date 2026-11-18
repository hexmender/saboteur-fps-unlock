# RE helper scripts

Python 3 + `pip install pefile capstone`. All take the path to `Saboteur.exe` first.

- `scan.py EXE` - timing imports (QPC/Sleep/...) and 1/60, 60.0, 1/30 constants
- `refs.py EXE ADDR...` - instructions referencing IAT slots / addresses
- `callers.py EXE FUNC...` - direct `call`/`jmp` sites of a function (+ raw pointer refs)
- `memrefs.py EXE ADDR...` - instructions with a memory operand at ADDR
- `sabdis.py EXE START END [pc_symbol_map.tsv]` - disassemble a range, annotate imports, float constants and known symbol names
- `templates.py PACK [types...]` - parse GameTemplates.wsd (AULB) embedded in `France\loosefiles_BinPC.pack` and dump camera properties (format per the Mercenaries-Fan-Build notes)
- `tension_patch.py PACK [MIN]` - **experimental**: raise every `CameraMoveTension`/`CameraAimTension` component below MIN (default 1.0, the engine default) to MIN, in place (back up the pack first)
- `fieldrefs.py EXE START END OFF...` - instructions in a range that touch `[reg + OFF]`
- `strgrep.py EXE REGEX` / `imports.py EXE REGEX [strings...]` - string and import search
