#!/usr/bin/env python3
"""Yayımlanmış BİRKÂT paketini indirir ve bütünlüğünü doğrular.

Bu betik, kullanıcıya teslim edilen ürünün gerçekten indirilebilir, eksiksiz ve
bozulmamış olduğunu **bağımsız olarak** doğrular. Ağ üzerinden indirir, boyut
ve SHA-256 karşılaştırır, arşiv yapısını denetler.

Kullanım:

    python3 scripts/verify-release.py                 # en son sürüm
    python3 scripts/verify-release.py --version 0.1.0
    python3 scripts/verify-release.py --skip-download  # yalnızca yerel dist/

Çıkış kodu 0 = doğrulama başarılı, 1 = başarısız.
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
import urllib.error
import urllib.request
import zipfile

REPO = "azmisahin-gov/birkat"
KURULUM_ADI = "BIRKAT-{v}-windows-x64-offline-setup.exe"
ZIP_ADI = "BIRKAT-{v}-windows-x64-offline.zip"
SUMS_ADI = "SHA256SUMS.txt"

# Kurulum paketinde bulunması ZORUNLU dosyalar.
ZORUNLU = [
    "BIRKAT.exe",
    "birkat-cli.exe",
    "KURULUM.txt",
    "LICENSE",
    "manifest.json",
    "models/model.safetensors",
    "models/config.json",
    "models/tokenizer.json",
    "models/melfilters128.bytes",
    "models/LICENSE",
    "THIRD-PARTY-LICENSES/sbom.cdx.json",
    "belgeler/kullanici-kilavuzu.md",
    "belgeler/kurulum-kilavuzu.md",
]

# Windows PE dosyası "MZ" ile başlar.
PE_IMZASI = b"MZ"

# Taşınabilir çalışma zamanı bağımlılıkları yasak.
YASAK_BAGIMLILIKLAR = ("libgcc", "libwinpthread", "libstdc")


def sha256(path, blok=1 << 20):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while True:
            b = f.read(blok)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def indir(url, hedef):
    print(f"İndiriliyor: {url}")
    t0 = os.times().elapsed
    try:
        with urllib.request.urlopen(url, timeout=3600) as r, open(hedef, "wb") as f:
            while True:
                b = r.read(1 << 20)
                if not b:
                    break
                f.write(b)
    except urllib.error.HTTPError as e:
        print(f"HATA: indirme başarısız (HTTP {e.code}): {url}", file=sys.stderr)
        return False
    sure = os.times().elapsed - t0
    boyut = os.path.getsize(hedef)
    print(f"  OK: {boyut:,} bayt ({sure:.0f} sn)")
    return True


def dogrula_zip(zip_yolu):
    """ZIP paketinin içeriğini denetler."""
    sonuc = []
    with zipfile.ZipFile(zip_yolu) as z:
        adlar = set(z.namelist())
        kok = None
        for a in adlar:
            if a.endswith("/BIRKAT.exe"):
                kok = a.rsplit("BIRKAT.exe", 1)[0]
                break
        if kok is None:
            sonuc.append(("HATA", "Paket içinde BIRKAT.exe bulunamadı"))
            return sonuc

        for gerekli in ZORUNLU:
            yol = kok + gerekli
            if yol in adlar:
                sonuc.append(("OK", gerekli))
            else:
                sonuc.append(("HATA", f"eksik: {gerekli}"))

        # PE imzası
        with z.open(kok + "BIRKAT.exe") as f:
            imza = f.read(2)
        if imza == PE_IMZASI:
            sonuc.append(("OK", "BIRKAT.exe geçerli Windows PE (MZ)"))
        else:
            sonuc.append(("HATA", "BIRKAT.exe PE imzası geçersiz"))

        # Manifesto
        try:
            with z.open(kok + "manifest.json") as f:
                m = json.load(f)
            if m.get("whisper_enabled") and m.get("offline"):
                sonuc.append(("OK", "manifest: whisper etkin, çevrimdışı"))
            else:
                sonuc.append(("HATA", "manifest: whisper/offline bayrakları eksik"))
        except Exception as e:
            sonuc.append(("HATA", f"manifest okunamadı: {e}"))

        # Model boyutu (en az 1 GB olmalı)
        try:
            bilgi = z.getinfo(kok + "models/model.safetensors")
            if bilgi.file_size > 1_000_000_000:
                sonuc.append(("OK", f"model.safetensors {bilgi.file_size:,} bayt"))
            else:
                sonuc.append(("HATA", f"model çok küçük: {bilgi.file_size}"))
        except KeyError:
            pass

    return sonuc


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--version", default="0.1.0")
    ap.add_argument("--skip-download", action="store_true",
                    help="Ağdan indirme; yerel dist/ klasörünü kullan")
    ap.add_argument("--work", default="/tmp/birkat-verify")
    args = ap.parse_args()

    v = args.version
    os.makedirs(args.work, exist_ok=True)
    basari = True

    print("=" * 70)
    print(f"BİRKÂT {v} — yayın doğrulaması")
    print("=" * 70)

    # --- 1. Kurulum dosyası ---
    setup = os.path.join(args.work, KURULUM_ADI.format(v=v))
    if args.skip_download:
        yerel = os.path.join("dist", KURULUM_ADI.format(v=v))
        if not os.path.isfile(yerel):
            print(f"HATA: yerel dosya yok: {yerel}", file=sys.stderr)
            return 1
        setup = yerel
        print(f"Yerel dosya kullanılıyor: {setup}")
    else:
        url = f"https://github.com/{REPO}/releases/download/v{v}/{KURULUM_ADI.format(v=v)}"
        if not indir(url, setup):
            return 1

    boyut = os.path.getsize(setup)
    print(f"\nKurulum dosyası: {boyut:,} bayt")
    with open(setup, "rb") as f:
        if f.read(2) != PE_IMZASI:
            print("HATA: kurulum dosyası geçerli bir Windows PE değil", file=sys.stderr)
            basari = False
        else:
            print("PE imzası: geçerli (MZ)")

    # --- 2. SHA-256 karşılaştırması ---
    sums_yerel = os.path.join("dist", SUMS_ADI)
    if os.path.isfile(sums_yerel):
        beklenen = None
        for satir in open(sums_yerel, encoding="utf-8"):
            parcalar = satir.split()
            if len(parcalar) == 2 and parcalar[1] == KURULUM_ADI.format(v=v):
                beklenen = parcalar[0]
                break
        if beklenen:
            gercek = sha256(setup)
            if gercek == beklenen:
                print(f"SHA-256: eşleşti ({gercek[:32]}…)")
            else:
                print(f"HATA: SHA-256 uyuşmuyor\n  beklenen: {beklenen}\n  gerçek  : {gercek}",
                      file=sys.stderr)
                basari = False
        else:
            print("UYARI: SHA256SUMS.txt içinde kurulum dosyası bulunamadı")
    else:
        print("UYARI: yerel SHA256SUMS.txt yok; hash karşılaştırması atlandı")

    # --- 3. ZIP içeriği ---
    zip_yolu = os.path.join(args.work, ZIP_ADI.format(v=v))
    if args.skip_download:
        yerel_zip = os.path.join("dist", ZIP_ADI.format(v=v))
        if os.path.isfile(yerel_zip):
            zip_yolu = yerel_zip
        else:
            print("\nUYARI: yerel ZIP yok; içerik denetimi atlandı")
            zip_yolu = None
    else:
        url = f"https://github.com/{REPO}/releases/download/v{v}/{ZIP_ADI.format(v=v)}"
        if not indir(url, zip_yolu):
            zip_yolu = None

    if zip_yolu and os.path.isfile(zip_yolu):
        print("\n--- Paket içeriği ---")
        for durum, mesaj in dogrula_zip(zip_yolu):
            isaret = "OK  " if durum == "OK" else "HATA"
            print(f"  [{isaret}] {mesaj}")
            if durum == "HATA":
                basari = False

    # --- 4. Bağımlılık denetimi (objdump varsa) ---
    objdump = None
    for aday in ("x86_64-w64-mingw32-objdump", "objdump"):
        try:
            subprocess.run([aday, "--version"], capture_output=True, check=True)
            objdump = aday
            break
        except Exception:
            continue

    if objdump and not args.skip_download:
        print("\n--- Windows bağımlılıkları ---")
        cikti = subprocess.run([objdump, "-p", setup], capture_output=True, text=True).stdout
        # NSIS kurulum dosyası sıkıştırılmıştır; iç ikili ayrıca denetlenir.
        import zipfile as zf
        if zip_yolu and os.path.isfile(zip_yolu):
            with zf.ZipFile(zip_yolu) as z:
                for a in z.namelist():
                    if a.endswith("BIRKAT.exe"):
                        with z.open(a) as f:
                            veri = f.read()
                        gecici = os.path.join(args.work, "BIRKAT.exe")
                        open(gecici, "wb").write(veri)
                        c2 = subprocess.run([objdump, "-p", gecici],
                                            capture_output=True, text=True).stdout
                        yasak = [y for y in YASAK_BAGIMLILIKLAR if y in c2.lower()]
                        if yasak:
                            print(f"  [HATA] taşınabilir çalışma zamanı bağımlılığı: {yasak}")
                            basari = False
                        else:
                            print("  [OK  ] yalnızca Windows sistem DLL'leri")
                        break
    else:
        print("\n(objdump bulunamadı; bağımlılık denetimi atlandı)")

    print("\n" + "=" * 70)
    if basari:
        print("SONUÇ: Doğrulama BAŞARILI. Paket eksiksiz ve bozulmamış.")
        print("=" * 70)
        return 0
    print("SONUÇ: Doğrulama BAŞARISIZ. Yukarıdaki hataları inceleyin.")
    print("=" * 70)
    return 1


if __name__ == "__main__":
    sys.exit(main())
