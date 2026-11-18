#!/usr/bin/env bash
# BİRKÂT Windows x64 kurulum paketi üretir.
#
# Kullanım:
#   ./scripts/package-windows.sh [--model-dir /yol/model]
#
# Gereksinimler:
#   - Windows x64 hedefi kurulu Rust toolchain
#   - (Opsiyonel) NSIS: makensis
#   - Model dosyaları (--model-dir ile verilir)
#
# Çıktı: dist/BIRKAT-<sürüm>-win64/ ve (NSIS varsa) .exe kurulum paketi.

set -euo pipefail

VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/' || echo "0.1.0")
TARGET="x86_64-pc-windows-gnu"
DIST="dist"
PKG="$DIST/BIRKAT-$VERSION-win64"
MODEL_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model-dir) MODEL_DIR="$2"; shift 2 ;;
    *) echo "Bilinmeyen argüman: $1"; exit 1 ;;
  esac
done

echo "== BİRKÂT $VERSION Windows x64 paketi =="

# 1. Windows hedefiyle derle
echo "-- Derleniyor ($TARGET, release) --"
if ! rustup target list --installed | grep -q "$TARGET"; then
  echo "Hedef kurulu değil; ekleniyor..."
  rustup target add "$TARGET"
fi

cargo build --release --target "$TARGET" \
  -p birkat-desktop -p birkat-cli

# 2. Paket klasörünü oluştur
echo "-- Paket klasörü hazırlanıyor --"
rm -rf "$PKG"
mkdir -p "$PKG/belgeler" "$PKG/THIRD-PARTY-LICENSES" "$PKG/models"

cp "target/$TARGET/release/birkat-desktop.exe" "$PKG/"
cp "target/$TARGET/release/birkat.exe" "$PKG/"

# 3. Belgeler
cp README.md LICENSE "$PKG/" 2>/dev/null || true
cp docs/*.md "$PKG/belgeler/" 2>/dev/null || true
cp docs/sbom.cdx.json "$PKG/belgeler/" 2>/dev/null || true

# 4. Lisanslar
echo "-- Lisanslar toplanıyor --"
if command -v cargo-license >/dev/null 2>&1; then
  cargo license --all-features > "$PKG/THIRD-PARTY-LICENSES/LICENSES.txt" || true
else
  cp docs/bagimlilik-lisanslari.md "$PKG/THIRD-PARTY-LICENSES/LICENSES.md" 2>/dev/null || true
fi

# 5. Model paketi
if [[ -n "$MODEL_DIR" && -d "$MODEL_DIR" ]]; then
  echo "-- Model paketi kopyalanıyor: $MODEL_DIR --"
  cp -r "$MODEL_DIR"/. "$PKG/models/"
else
  cat > "$PKG/models/OKU-BENI.txt" <<'EOF'
Bu klasöre Whisper large-v3-turbo model dosyaları konmalıdır:
  - model.safetensors
  - config.json
  - tokenizer.json
  - melfilters.bytes
  - LICENSE

Model dosyaları telif nedeniyle depoda bulunmaz. Paketlemek için:
  ./scripts/package-windows.sh --model-dir /yol/model

Model ilk çalıştırmada internetten İNDİRİLMEZ; kurulum paketinde
bulunmalıdır. Bkz. belgeler/kurulum-kilavuzu.md
EOF
  echo "UYARI: Model dizini verilmedi; models/ içine OKU-BENI.txt yazıldı."
fi

# 6. Paket manifestosu (hash'ler)
echo "-- Paket manifestosu üretiliyor --"
python3 - "$PKG" <<'PYEOF'
import hashlib, json, os, sys
pkg = sys.argv[1]
entries = []
for root, _dirs, files in os.walk(pkg):
    for f in sorted(files):
        p = os.path.join(root, f)
        rel = os.path.relpath(p, pkg).replace(os.sep, '/')
        h = hashlib.sha256()
        size = 0
        with open(p, 'rb') as fh:
            while True:
                b = fh.read(1 << 20)
                if not b:
                    break
                h.update(b)
                size += len(b)
        entries.append({'path': rel, 'size_bytes': size, 'sha256': h.hexdigest()})
manifest = {
    'package': os.path.basename(pkg),
    'version': os.environ.get('VERSION', '0.1.0'),
    'platform': 'windows-x64',
    'files': entries,
    'total_bytes': sum(e['size_bytes'] for e in entries),
}
with open(os.path.join(pkg, 'manifest.json'), 'w', encoding='utf-8') as f:
    json.dump(manifest, f, ensure_ascii=False, indent=2)
print(f"manifest.json: {len(entries)} dosya, {manifest['total_bytes']} bayt")
PYEOF

# 7. NSIS kurulum paketi (varsa)
if command -v makensis >/dev/null 2>&1; then
  echo "-- NSIS kurulum paketi üretiliyor --"
  cat > "$DIST/birkat.nsi" <<NSIEOF
!define APPNAME "BIRKAT"
!define VERSION "$VERSION"
Name "\${APPNAME} \${VERSION}"
OutFile "$DIST/BIRKAT-$VERSION-win64.exe"
InstallDir "\$PROGRAMFILES64\\BIRKAT"
RequestExecutionLevel admin
Page directory
Page instfiles
Section
  SetOutPath "\$INSTDIR"
  File /r "$PKG\\*.*"
  CreateShortcut "\$SMPROGRAMS\\BIRKAT.lnk" "\$INSTDIR\\birkat-desktop.exe"
  CreateShortcut "\$DESKTOP\\BIRKAT.lnk" "\$INSTDIR\\birkat-desktop.exe"
SectionEnd
NSIEOF
  makensis "$DIST/birkat.nsi" || echo "NSIS üretimi başarısız; klasör paketi kullanılabilir."
else
  echo "NSIS bulunamadı; klasör paketi üretildi: $PKG"
  echo "Kurulum paketi için NSIS kurun (makensis)."
fi

echo ""
echo "== Tamamlandı =="
echo "Paket klasörü: $PKG"
ls -la "$PKG" | head -20
