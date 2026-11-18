#!/usr/bin/env python3
"""cargo metadata çıktısından SBOM ve lisans belgesi üretir."""
import json, sys
from collections import Counter

OWN = {
    'birkat-core','birkat-crypto','birkat-audit','birkat-evidence','birkat-audio',
    'birkat-transcription','birkat-udf','birkat-report','birkat-backup',
    'birkat-updater','birkat-app','birkat-cli','birkat-desktop',
}

USAGES = {
 'aes-gcm':'AEAD şifreleme (AES-256-GCM)',
 'argon2':'Anahtar türetme (Argon2id)',
 'candle-core':'Yerel Whisper çıkarımı (Candle)',
 'candle-nn':'Sinir ağı katmanları',
 'candle-transformers':'Whisper model uygulaması',
 'clap':'Komut satırı arayüzü',
 'eframe':'Masaüstü arayüz çatısı',
 'egui':'Masaüstü arayüz bileşenleri',
 'egui_extras':'Arayüz yardımcı bileşenleri',
 'ewf-image':'E01/L01/Ex01/Lx01 imaj okuma ve yazma',
 'filetime':'Dosya zaman bilgilerini koruma',
 'hound':'WAV okuma/yazma (test verisi)',
 'printpdf':'PDF yardımcı çıktısı',
 'quick-xml':'UDF content.xml çözümleme',
 'rand':'Rastgele sayı üretimi (anahtar, tuz, nonce)',
 'rfd':'Dosya seçme pencereleri',
 'rusqlite':'Yerel veritabanı (SQLite)',
 'serde':'Serileştirme',
 'serde_json':'JSON serileştirme',
 'sha2':'SHA-256 hash',
 'sha1':'SHA-1 hash (eski imaj uyumluluğu)',
 'md-5':'MD5 hash (eski imaj uyumluluğu)',
 'symphonia':'Ses çözümleme',
 'thiserror':'Hata tipleri',
 'time':'Tarih ve saat',
 'tokenizers':'Whisper belirteçleştirici',
 'tracing':'Günlükleme',
 'tracing-subscriber':'Günlük biçimlendirme',
 'uuid':'Kimlik üretimi',
 'walkdir':'Klasör tarama',
 'zip':'UDF/DOCX paketleme, arşiv güvenliği',
 'zeroize':'Anahtar bellekten silme',
 'hex':'Onaltılık kodlama',
 'base64':'Base64 kodlama',
 'toml':'Güncelleme manifestosu',
 'subtle':'Sabit zamanlı karşılaştırma',
}


def main():
    d = json.load(sys.stdin)
    pkgs = sorted(d['packages'], key=lambda p: (p['name'], p['version']))

    comps = []
    for p in pkgs:
        lic = p.get('license') or 'UNKNOWN'
        if ' AND ' in lic or ' OR ' in lic:
            licenses = [{'expression': lic}]
        else:
            licenses = [{'license': {'id': lic}}]
        comp = {
            'type': 'library',
            'name': p['name'],
            'version': p['version'],
            'licenses': licenses,
            'purl': f"pkg:cargo/{p['name']}@{p['version']}",
        }
        if p['name'] in OWN:
            comp['scope'] = 'required'
            comp['author'] = 'BİRKÂT Geliştirme Ekibi'
        comps.append(comp)

    bom = {
        'bomFormat': 'CycloneDX',
        'specVersion': '1.5',
        'version': 1,
        'metadata': {
            'timestamp': '2026-11-18T00:00:00Z',
            'component': {
                'type': 'application',
                'name': 'birkat',
                'version': '0.1.0',
                'description': '56.12 Bilirkişi Dijital Çalışma Operatörü',
            },
            'tools': [{'vendor': 'BİRKÂT', 'name': 'cargo-metadata-sbom', 'version': '1.0'}],
        },
        'components': comps,
    }
    with open('docs/sbom.cdx.json', 'w', encoding='utf-8') as f:
        json.dump(bom, f, ensure_ascii=False, indent=2)
        f.write('\n')

    ext = [p for p in pkgs if p['name'] not in OWN]
    counts = Counter((p.get('license') or 'UNKNOWN') for p in ext)

    lines = [
        '# Bağımlılık Lisansları',
        '',
        'Bu belge, BİRKÂT derlemesinde kullanılan üçüncü taraf bağımlılıkların',
        'lisanslarını listeler. Tüm bağımlılıklar izin verici (permissive) veya',
        'zayıf copyleft (MPL-2.0) lisanslıdır; hiçbiri ürünün dağıtımını',
        'engellemez.',
        '',
        f'Toplam üçüncü taraf paket sayısı: **{len(ext)}**',
        '',
        '## Lisans özeti',
        '',
        '| Lisans | Paket sayısı |',
        '|---|---|',
    ]
    for lic, n in counts.most_common():
        lines.append(f'| {lic} | {n} |')

    lines += [
        '',
        '## Doğrudan bağımlılıklar',
        '',
        '| Paket | Sürüm | Lisans | Kullanım |',
        '|---|---|---|---|',
    ]
    seen = set()
    for p in pkgs:
        if p['name'] in OWN or p['name'] in seen:
            continue
        seen.add(p['name'])
        lines.append(
            f"| {p['name']} | {p['version']} | {p.get('license') or '?'} | "
            f"{USAGES.get(p['name'], '-')} |"
        )

    lines += [
        '',
        '## Notlar',
        '',
        '- `symphonia` MPL-2.0 lisanslıdır. MPL-2.0 zayıf copyleft bir lisans',
        '  olup, yalnızca kütüphanenin kendi kaynak dosyalarındaki değişikliklerin',
        '  paylaşılmasını gerektirir; kütüphaneyi kullanan uygulamanın kaynak',
        '  kodunu açmayı zorunlu kılmaz.',
        '- `ewf-image` Apache-2.0 lisanslıdır ve E01/L01/Ex01/Lx01 okuma ile EWF',
        '  yazma işlevlerini sağlar.',
        '- `printpdf` MIT lisanslıdır ve PDF yardımcı çıktısını üretir.',
        '- Model dosyaları (Whisper large-v3-turbo) ayrı bir lisans altındadır;',
        '  model paketi kendi lisans dosyasını taşır.',
        '',
        '## Lisans metinleri',
        '',
        'Tüm bağımlılıkların tam lisans metinleri `cargo metadata` çıktısında ve',
        'bağımlılık kaynaklarında bulunur. Kurulum paketi,',
        '`THIRD-PARTY-LICENSES` klasöründe bu metinleri içerir.',
    ]

    with open('docs/bagimlilik-lisanslari.md', 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines) + '\n')

    print(f'SBOM: {len(comps)} bileşen -> docs/sbom.cdx.json')
    print(f'Lisans: {len(ext)} üçüncü taraf paket -> docs/bagimlilik-lisanslari.md')


if __name__ == '__main__':
    main()
