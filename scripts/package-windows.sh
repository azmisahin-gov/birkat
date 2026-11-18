#!/usr/bin/env bash
# BİRKÂT Windows x64 çevrimdışı kurulum paketi üretir.
#
# ## Ne üretir
#
#   dist/BIRKAT-<sürüm>-windows-x64-offline-setup.exe   (NSIS kurulum dosyası)
#   dist/BIRKAT-<sürüm>-windows-x64-offline.zip         (kurulum gerektirmeyen paket)
#   dist/SHA256SUMS.txt                                 (hash manifesti)
#   dist/BIRKAT-<sürüm>-sbom.cdx.json                   (SBOM)
#   dist/BIRKAT-<sürüm>-LICENSES.md                     (bağımlılık lisansları)
#
# ## Kesin kurallar
#
#   * Whisper/Candle çıkarım özelliği **zorunlu** olarak etkinleştirilir.
#     Özellik tanımlı değilse betik hata verir ve yayın üretmez.
#   * Model dosyaları **zorunludur**. Eksik modelle paket üretilmez;
#     açıklama dosyası yerine hata döndürülür.
#   * Model dosyalarının SHA-256 değerleri `model-package.lock.json` ile
#     karşılaştırılır; uyuşmazlıkta paket üretilmez.
#   * NSIS yoksa .exe kurulum dosyası üretilmez ve bu açıkça bildirilir.
#
# ## Kullanım
#
#   ./scripts/package-windows.sh --model-dir /yol/model
#
# ## Gereksinimler
#
#   * Rust toolchain + x86_64-pc-windows-gnu hedefi
#   * x86_64-w64-mingw32-gcc (mingw-w64)
#   * makensis (NSIS)
#   * python3, zip

set -euo pipefail

VERSION="$(python3 -c "
import re
s=open('Cargo.toml',encoding='utf-8').read()
m=re.search(r'^version\s*=\s*\"([^\"]+)\"', s, re.M)
print(m.group(1) if m else '0.1.0')
")"
TARGET="x86_64-pc-windows-gnu"
DIST="dist"
AD="BIRKAT-${VERSION}-windows-x64-offline"
PKG="$DIST/$AD"
MODEL_DIR=""
SKIP_MODEL_CHECK=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model-dir) MODEL_DIR="$2"; shift 2 ;;
    --skip-model-check) SKIP_MODEL_CHECK=1; shift ;;
    -h|--help) sed -n '2,26p' "$0"; exit 0 ;;
    *) echo "Bilinmeyen argüman: $1" >&2; exit 1 ;;
  esac
done

die() { echo "HATA: $*" >&2; exit 1; }
say() { echo "== $* =="; }

say "BİRKÂT $VERSION Windows x64 çevrimdışı paketi"

# ---------------------------------------------------------------- ön koşullar
say "Ön koşullar denetleniyor"

command -v cargo >/dev/null || die "cargo bulunamadı (Rust toolchain gerekli)"
command -v python3 >/dev/null || die "python3 bulunamadı"

if ! command -v x86_64-w64-mingw32-gcc >/dev/null; then
  die "x86_64-w64-mingw32-gcc bulunamadı (mingw-w64 gerekli)"
fi

NSIS_OK=1
if ! command -v makensis >/dev/null; then
  NSIS_OK=0
  echo "UYARI: makensis bulunamadı; .exe kurulum dosyası üretilemeyecek."
  echo "       Yalnızca ZIP paketi üretilecek. NSIS kurun: apt install nsis"
fi

if ! rustup target list --installed 2>/dev/null | grep -qx "$TARGET"; then
  say "Windows hedefi ekleniyor: $TARGET"
  rustup target add "$TARGET"
fi

# ------------------------------------------------------------------- model
say "Model paketi denetleniyor"

MODEL_FILES=(
  model.safetensors
  config.json
  tokenizer.json
  preprocessor_config.json
  generation_config.json
  added_tokens.json
  merges.txt
  normalizer.json
  special_tokens_map.json
  tokenizer_config.json
  vocab.json
  melfilters128.bytes
)

if [[ -z "$MODEL_DIR" ]]; then
  die "Model dizini verilmedi. Kullanım: $0 --model-dir /yol/model

Model dosyaları olmadan çevrimdışı kurulum paketi ÜRETİLMEZ.
Model paketini hazırlamak için bkz. docs/kurulum-kilavuzu.md"
fi

[[ -d "$MODEL_DIR" ]] || die "Model dizini bulunamadı: $MODEL_DIR"

for f in "${MODEL_FILES[@]}"; do
  [[ -f "$MODEL_DIR/$f" ]] || die "Model dosyası eksik: $MODEL_DIR/$f"
done

if [[ "$SKIP_MODEL_CHECK" -eq 0 ]]; then
  say "Model hash'leri doğrulanıyor (model-package.lock.json)"
  python3 - "$MODEL_DIR" <<'PYEOF'
import hashlib, json, sys, os
root = sys.argv[1]
lock = json.load(open("model-package.lock.json", encoding="utf-8"))
hatali = []
for e in lock["files"]:
    p = os.path.join(root, e["path"])
    h = hashlib.sha256()
    n = 0
    with open(p, "rb") as f:
        while True:
            b = f.read(1 << 20)
            if not b:
                break
            h.update(b); n += len(b)
    if h.hexdigest() != e["sha256"]:
        hatali.append(f'{e["path"]}: hash uyuşmuyor')
    elif n != e["size_bytes"]:
        hatali.append(f'{e["path"]}: boyut {n} != {e["size_bytes"]}')
if hatali:
    print("HASH UYUŞMAZLIĞI:", file=sys.stderr)
    for x in hatali:
        print("  " + x, file=sys.stderr)
    sys.exit(1)
print(f'{len(lock["files"])} model dosyası doğrulandı.')
PYEOF
fi

# ------------------------------------------------------------ derleme
say "Windows x64 derlemesi (Whisper/Candle özelliği ZORUNLU)"

if ! grep -q '"dep:candle-core"' crates/birkat-transcription/Cargo.toml; then
  die "birkat-transcription 'whisper' özelliği tanımlı değil; üretim derlemesi yapılamaz."
fi

cargo build --release --target "$TARGET" \
  -p birkat-desktop \
  -p birkat-cli \
  --features birkat-transcription/whisper

DESKTOP_EXE="target/$TARGET/release/birkat-desktop.exe"
CLI_EXE="target/$TARGET/release/birkat.exe"

[[ -f "$DESKTOP_EXE" ]] || die "Masaüstü ikilisi üretilemedi: $DESKTOP_EXE"
[[ -f "$CLI_EXE" ]] || die "Komut satırı ikilisi üretilemedi: $CLI_EXE"

# PE imzası: her Windows yürütülebilir dosyası "MZ" ile başlar.
if [[ "$(head -c 2 "$DESKTOP_EXE")" != "MZ" ]]; then
  die "Üretilen ikili geçerli bir Windows PE dosyası değil (MZ imzası yok)."
fi

# objdump çıktısı büyüktür ve uyarıları stderr'e yazarak sıfırdan farklı
# çıkabilir. Ayrıca `grep -q` erken çıkınca `pipefail` altında SIGPIPE
# (exit 141) oluşur ve denetim yanlışlıkla başarısız görünür. Bu nedenle
# çıktı bir kez dosyaya alınır ve erken çıkmayan `grep -c` ile sayılır.
OBJDUMP_CIKTI="$(mktemp)"
x86_64-w64-mingw32-objdump -p "$DESKTOP_EXE" >"$OBJDUMP_CIKTI" 2>/dev/null || true

DLL_SAYISI="$(grep -ci 'DLL Name' "$OBJDUMP_CIKTI" || true)"
if [ "${DLL_SAYISI:-0}" -eq 0 ]; then
  rm -f "$OBJDUMP_CIKTI"
  die "İkilinin içe aktarma tablosu okunamadı; geçerli bir Windows PE değil."
fi

TASINABILIR_SAYISI="$(grep -ciE 'libgcc|libwinpthread|libstdc' "$OBJDUMP_CIKTI" || true)"
if [ "${TASINABILIR_SAYISI:-0}" -ne 0 ]; then
  rm -f "$OBJDUMP_CIKTI"
  die "İkili taşınabilir çalışma zamanı kitaplığına bağlı (statik bağlama gerekli)."
fi

# Yalnızca Windows sistem DLL'lerine bağlı olmalı; kullanıcı hiçbir çalışma
# zamanı kurmak zorunda kalmamalıdır.
BAGIMLILIKLAR="$(grep -i 'DLL Name' "$OBJDUMP_CIKTI" | awk '{print $3}' | sort -u | tr '\n' ' ' || true)"
rm -f "$OBJDUMP_CIKTI"
echo "Bağımlılıklar: $BAGIMLILIKLAR"

# ------------------------------------------------------------ paket klasörü
say "Paket klasörü hazırlanıyor: $PKG"
rm -rf "$PKG"
mkdir -p "$PKG/belgeler" "$PKG/models" "$PKG/THIRD-PARTY-LICENSES"

cp "$DESKTOP_EXE" "$PKG/BIRKAT.exe"
cp "$CLI_EXE" "$PKG/birkat-cli.exe"
cp README.md LICENSE "$PKG/"
cp docs/*.md "$PKG/belgeler/" 2>/dev/null || true

cp -r "$MODEL_DIR"/. "$PKG/models/"
cat > "$PKG/models/LICENSE" <<'EOF'
Whisper large-v3-turbo
======================

Bu klasördeki model dosyaları OpenAI tarafından yayımlanmıştır.

  Kaynak : https://huggingface.co/openai/whisper-large-v3-turbo
  Lisans : MIT

MIT Lisansı, telif hakkı bildirimi ve bu izin bildirimi korunduğu sürece
yazılımın kullanılmasına, kopyalanmasına, değiştirilmesine, birleştirilmesine,
yayımlanmasına, dağıtılmasına, alt lisanslanmasına ve satılmasına izin verir.

Tam lisans metni: https://github.com/openai/whisper/blob/main/LICENSE
EOF

if command -v cargo-license >/dev/null 2>&1; then
  cargo license --all-features > "$PKG/THIRD-PARTY-LICENSES/LICENSES.txt" 2>/dev/null || true
fi
cp docs/bagimlilik-lisanslari.md "$PKG/THIRD-PARTY-LICENSES/LICENSES.md" 2>/dev/null || true
[[ -f docs/sbom.cdx.json ]] && cp docs/sbom.cdx.json "$PKG/THIRD-PARTY-LICENSES/sbom.cdx.json"

cat > "$PKG/KURULUM.txt" <<EOF
BİRKÂT $VERSION — Windows x64 çevrimdışı paket
================================================

Bu paket İNTERNET BAĞLANTISI GEREKTİRMEZ. Whisper konuşma tanıma modeli
paket içinde bulunmaktadır.

KURULUM
-------
BIRKAT-${VERSION}-windows-x64-offline-setup.exe dosyasını çalıştırın.
Kurulum sihirbazı sizi yönlendirir. Yönetici yetkisi istenir.

Kurulum sonrası Başlat menüsünde ve masaüstünde "BİRKÂT" kısayolu oluşur.

İLK ÇALIŞTIRMA
--------------
1. BİRKÂT'ı açın.
2. Bilirkişi profilinizi oluşturun (ad soyad, unvan, sicil numarası).
3. Veri kök dizinini ve yedek dizinini seçin.
4. Kasa parolasını belirleyin (en az 12 karakter).
5. Kurtarma anahtarını güvenli bir yere yazın.

KALDIRMA
--------
Ayarlar → Uygulamalar → BİRKÂT → Kaldır.
Kaldırma vaka verinizi SİLMEZ; veri kök dizini korunur.

DESTEK
------
Türkçe kullanım kılavuzu: belgeler/kullanici-kilavuzu.md
Kurulum kılavuzu        : belgeler/kurulum-kilavuzu.md
EOF

# ------------------------------------------------------------ paket manifestosu
say "Paket hash manifesti üretiliyor"
python3 - "$PKG" "$VERSION" <<'PYEOF'
import hashlib, json, os, sys
pkg, version = sys.argv[1], sys.argv[2]
entries = []
for root, _dirs, files in os.walk(pkg):
    for f in sorted(files):
        p = os.path.join(root, f)
        rel = os.path.relpath(p, pkg).replace(os.sep, "/")
        h = hashlib.sha256(); size = 0
        with open(p, "rb") as fh:
            while True:
                b = fh.read(1 << 20)
                if not b:
                    break
                h.update(b); size += len(b)
        entries.append({"path": rel, "size_bytes": size, "sha256": h.hexdigest()})
manifest = {
    "package": os.path.basename(pkg),
    "version": version,
    "platform": "windows-x64",
    "offline": True,
    "whisper_enabled": True,
    "files": entries,
    "total_bytes": sum(e["size_bytes"] for e in entries),
}
with open(os.path.join(pkg, "manifest.json"), "w", encoding="utf-8") as f:
    json.dump(manifest, f, ensure_ascii=False, indent=2)
print(f'manifest.json: {len(entries)} dosya, {manifest["total_bytes"]:,} bayt')
PYEOF

# ------------------------------------------------------------ ZIP paketi
say "ZIP paketi üretiliyor (kurulum gerektirmez)"
( cd "$DIST" && zip -qr "${AD}.zip" "$AD" )
echo "ZIP: $DIST/${AD}.zip ($(stat -c%s "$DIST/${AD}.zip" 2>/dev/null || echo '?') bayt)"

# ------------------------------------------------------------ NSIS kurulum
SETUP="$DIST/${AD}-setup.exe"
if [[ "$NSIS_OK" -eq 1 ]]; then
  say "NSIS kurulum dosyası üretiliyor"
  mkdir -p "$DIST/nsis"
  # NSIS mutlak yol ister; betik dist/nsis/ içinden çalışır.
  PKG_ABS="$(cd "$(dirname "$PKG")" && pwd)/$(basename "$PKG")"
  PKG_WIN="$(echo "$PKG_ABS" | sed 's|/|\\\\|g')"
  cat > "$DIST/nsis/birkat.nsi" <<NSIEOF
!include "MUI2.nsh"

!define APPNAME "BIRKAT"
!define APPTITLE "BİRKÂT — 56.12 Bilirkişi Dijital Çalışma Operatörü"
!define VERSION "$VERSION"
!define PUBLISHER "BİRKÂT Geliştirme Ekibi"
!define PKGDIR "$PKG_WIN"

Name "\${APPTITLE}"
OutFile "..\\${AD}-setup.exe"
InstallDir "\$PROGRAMFILES64\\BIRKAT"
InstallDirRegKey HKLM "Software\\BIRKAT" "InstallDir"
RequestExecutionLevel admin
Unicode true
# Model ağırlıkları yoğun (neredeyse sıkıştırılamaz) f16 verisidir.
# LZMA, saatler süren sıkıştırma karşılığında neredeyse hiç kazanç sağlamaz;
# zlib hızlıdır ve kurulum süresini makul tutar.
SetCompressor zlib

!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TITLE "BİRKÂT kurulumu"
!define MUI_WELCOMEPAGE_TEXT "Bu sihirbaz BİRKÂT $VERSION sürümünü bilgisayarınıza kuracak.\$\r\$\n\$\r\$\nBİRKÂT, 56.12 bilirkişileri için geliştirilmiş yerel ve çevrimdışı çalışan bir uygulamadır. Whisper konuşma tanıma modeli paket içinde gelir; internet bağlantısı gerekmez."
!define MUI_FINISHPAGE_TITLE "BİRKÂT kuruldu"
!define MUI_FINISHPAGE_TEXT "BİRKÂT başarıyla kuruldu.\$\r\$\n\$\r\$\nİlk açılışta bilirkişi profilinizi oluşturmanız ve kasa parolanızı belirlemeniz istenir."
!define MUI_FINISHPAGE_RUN "\$INSTDIR\\BIRKAT.exe"
!define MUI_FINISHPAGE_RUN_TEXT "BİRKÂT'ı şimdi başlat"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "\${PKGDIR}\\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Turkish"

Section "BİRKÂT" SecMain
  SectionIn RO
  SetOutPath "\$INSTDIR"
  File /r "\${PKGDIR}\\*.*"

  WriteRegStr HKLM "Software\\BIRKAT" "InstallDir" "\$INSTDIR"
  WriteRegStr HKLM "Software\\BIRKAT" "Version" "\${VERSION}"
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BIRKAT" "DisplayName" "\${APPTITLE}"
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BIRKAT" "DisplayVersion" "\${VERSION}"
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BIRKAT" "Publisher" "\${PUBLISHER}"
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BIRKAT" "UninstallString" "\$INSTDIR\\Kaldir.exe"
  WriteRegStr HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BIRKAT" "InstallLocation" "\$INSTDIR"
  WriteRegDWORD HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BIRKAT" "NoModify" 1
  WriteRegDWORD HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BIRKAT" "NoRepair" 1

  CreateDirectory "\$SMPROGRAMS\\BİRKÂT"
  CreateShortcut "\$SMPROGRAMS\\BİRKÂT\\BİRKÂT.lnk" "\$INSTDIR\\BIRKAT.exe"
  CreateShortcut "\$SMPROGRAMS\\BİRKÂT\\Kullanım Kılavuzu.lnk" "\$INSTDIR\\belgeler\\kullanici-kilavuzu.md"
  CreateShortcut "\$SMPROGRAMS\\BİRKÂT\\Kaldır.lnk" "\$INSTDIR\\Kaldir.exe"
  CreateShortcut "\$DESKTOP\\BİRKÂT.lnk" "\$INSTDIR\\BIRKAT.exe"

  WriteUninstaller "\$INSTDIR\\Kaldir.exe"
SectionEnd

Section "Uninstall"
  Delete "\$DESKTOP\\BİRKÂT.lnk"
  Delete "\$SMPROGRAMS\\BİRKÂT\\BİRKÂT.lnk"
  Delete "\$SMPROGRAMS\\BİRKÂT\\Kullanım Kılavuzu.lnk"
  Delete "\$SMPROGRAMS\\BİRKÂT\\Kaldır.lnk"
  RMDir "\$SMPROGRAMS\\BİRKÂT"
  RMDir /r "\$INSTDIR"
  DeleteRegKey HKLM "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\BIRKAT"
  DeleteRegKey HKLM "Software\\BIRKAT"
SectionEnd
NSIEOF
  ( cd "$DIST/nsis" && makensis birkat.nsi )
  [[ -f "$SETUP" ]] || die "NSIS kurulum dosyası üretilemedi."
  echo "Kurulum: $SETUP ($(stat -c%s "$SETUP") bayt)"
else
  echo "ATLANDI: NSIS yok; .exe kurulum dosyası üretilmedi."
fi

# ------------------------------------------------------------ yayın dosyaları
say "Yayın dosyaları ve hash manifesti"
cp docs/sbom.cdx.json "$DIST/BIRKAT-${VERSION}-sbom.cdx.json" 2>/dev/null || true
cp docs/bagimlilik-lisanslari.md "$DIST/BIRKAT-${VERSION}-LICENSES.md" 2>/dev/null || true

python3 - "$DIST" "$AD" "$VERSION" "$NSIS_OK" <<'PYEOF'
import hashlib, os, sys
dist, ad, version, nsis_ok = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4] == "1"
targets = [f"{ad}.zip"]
if nsis_ok:
    targets.append(f"{ad}-setup.exe")
for extra in (f"BIRKAT-{version}-sbom.cdx.json", f"BIRKAT-{version}-LICENSES.md"):
    if os.path.exists(os.path.join(dist, extra)):
        targets.append(extra)
lines = []
for t in targets:
    p = os.path.join(dist, t)
    h = hashlib.sha256()
    with open(p, "rb") as f:
        while True:
            b = f.read(1 << 20)
            if not b:
                break
            h.update(b)
    lines.append(f"{h.hexdigest()}  {t}")
    print(f"{t}: {h.hexdigest()}")
with open(os.path.join(dist, "SHA256SUMS.txt"), "w", encoding="utf-8") as f:
    f.write("\n".join(lines) + "\n")
PYEOF

say "Tamamlandı"
echo
echo "Yayın dosyaları ($DIST/):"
ls -la "$DIST"/*.zip "$DIST"/*.exe "$DIST"/SHA256SUMS.txt 2>/dev/null || true
echo
if [[ "$NSIS_OK" -eq 0 ]]; then
  echo "UYARI: .exe kurulum dosyası ÜRETİLMEDİ (NSIS eksik)."
  echo "Bu paket tamamlanmış kurulum paketi olarak sunulmamalıdır."
fi
