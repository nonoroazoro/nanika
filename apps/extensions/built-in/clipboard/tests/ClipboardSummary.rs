use super::*;
use nanika_localization::{LocaleBundle, SupportedLocale};

#[test]
fn generated_labels_format_in_both_languages_without_changing_user_text() {
    let text = ClipboardSummary::Text {
        preview: "Image 8 x 9 用户原文".into()
    }
    .title()
    .unwrap();
    let single = ClipboardSummary::Files {
        first_path: "/old/Image 1 x 2 and 8 more.txt".into(),
        count: 1
    }
    .title()
    .unwrap();
    let files = ClipboardSummary::Files {
        first_path: "/old/report.txt".into(),
        count: 2
    }
    .title()
    .unwrap();
    let image = ClipboardSummary::Image {
        dimensions: crate::ClipboardImageDimensions {
            width: 1920,
            height: 1080
        }
    }
    .title()
    .unwrap();
    for (locale, source, expected_files, expected_image) in [
        (
            SupportedLocale::English,
            include_str!("../locales/en.ftl"),
            "report.txt and 1 more",
            "Image 1920 × 1080"
        ),
        (
            SupportedLocale::SimplifiedChinese,
            include_str!("../locales/zh-Hans.ftl"),
            "report.txt和另外 1 个文件",
            "图片 1920 × 1080"
        )
    ] {
        let bundle = LocaleBundle::new(locale, [("clipboard.ftl".into(), source.into())]).unwrap();
        for platform in ["macos", "windows"] {
            assert_eq!(
                bundle.format(&text, platform, 128, false).unwrap(),
                "Image 8 x 9 用户原文"
            );
            assert_eq!(
                bundle.format(&single, platform, 128, false).unwrap(),
                "Image 1 x 2 and 8 more.txt"
            );
            let rendered = bundle.format(&files, platform, 128, false).unwrap();
            assert!(rendered.contains('\u{2068}') && rendered.contains('\u{2069}'));
            assert_eq!(
                rendered.replace(['\u{2068}', '\u{2069}'], ""),
                expected_files
            );
            assert_eq!(
                bundle
                    .format(&image, platform, 128, false)
                    .unwrap()
                    .replace(['\u{2068}', '\u{2069}'], ""),
                expected_image
            );
            let long_name = ClipboardSummary::Files {
                first_path: format!("/old/{}\n\t.txt", "文件".repeat(100)),
                count: 256
            }
            .title()
            .unwrap();
            assert!(bundle.format(&long_name, platform, 128, false).is_ok());
        }
    }
}

#[test]
fn image_dimensions_are_required_only_for_image_payloads() {
    let mut entry = crate::ClipboardEntry {
        entry_id: "image".into(),
        image_dimensions: None,
        content: nanika_protocol::ClipboardContent::PngFile {
            path: "missing.png".into()
        },
        byte_size: 123,
        captured_at: 456
    };
    assert!(entry.summary().is_err());
    entry.image_dimensions = Some(crate::ClipboardImageDimensions {
        width: 8193,
        height: 1
    });
    assert!(entry.summary().is_err());
    entry.image_dimensions = Some(crate::ClipboardImageDimensions {
        width: 1920,
        height: 1080
    });
    assert!(entry.summary().is_ok());
    entry.content = nanika_protocol::ClipboardContent::Text {
        value: "Image 1920 x 1080".into()
    };
    assert!(entry.summary().is_err());
}
