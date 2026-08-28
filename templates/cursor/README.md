# Cursor templates

Tracked seeds (safe to commit):
`templates/cursor/User/{settings,keybindings}.json.example`

Live `settings.json` / `keybindings.json` under templates are gitignored —
Cursor edits must not dirty the product tree. On apply, missing User files are
**copied** from `.example` (not symlinked).

Personal overrides (gitignored): `overlay/cursor/User/*.json` (symlinked when present).
