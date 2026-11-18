//! PDF çıktısı üretir ve metin çıkarımı için dosyaya yazar.
use birkat_core::ids::CaseId;
use birkat_core::report::{Align, Block, ReportDocument, ReportKind, SectionId};
use birkat_report::pdf::write_pdf;

fn main() {
    let mut r = ReportDocument::new(CaseId::new(), ReportKind::Main, 1);
    r.push_block(
        SectionId::Subject,
        Block::Paragraph {
            text: "İnceleme konusu: ĞÜŞİÖÇ ğüşiöç çözümlemesi yapılması.".to_string(),
            align: Align::Justify,
        },
    );
    r.push_block(
        SectionId::Conclusion,
        Block::Paragraph {
            text: "Gerekçeli sonuç: Türkçe karakterler doğru görünmelidir.".to_string(),
            align: Align::Justify,
        },
    );
    let out = std::env::args()
        .nth(1)
        .unwrap_or("/tmp/birkat_test.pdf".to_string());
    write_pdf(&r, &out).unwrap();
    println!("yazıldı: {out}");
}
