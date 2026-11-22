use resumaker_core::export::{self, Lang, Options};
use resumaker_core::{Error, Format, Profile, Template};

fn render(format: Format) -> Vec<u8> {
    export::render(&Profile::example(), format, &Options::default()).unwrap()
}

fn render_str(format: Format) -> String {
    String::from_utf8(render(format)).unwrap()
}

#[test]
fn pdf_with_default_template() {
    let pdf = render(Format::Pdf);
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(
        pdf.len() > 10_000,
        "suspiciously small PDF: {} bytes",
        pdf.len()
    );
}

#[test]
fn pdf_in_german() {
    let options = Options {
        lang: Lang::De,
        ..Options::default()
    };
    let pdf = export::render(&Profile::example(), Format::Pdf, &options).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
}

#[test]
fn pdf_from_empty_profile() {
    let pdf = export::render(&Profile::default(), Format::Pdf, &Options::default()).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
}

#[test]
fn broken_template_reports_line() {
    let options = Options {
        template: Template::new("broken", "#let x = \n#undefined-function()"),
        ..Options::default()
    };
    let err = export::render(&Profile::example(), Format::Pdf, &options).unwrap_err();
    assert!(matches!(err, Error::Render(_)));
    assert!(err.to_string().contains("line "), "{err}");
}

#[test]
fn templates_cannot_read_the_disk() {
    let options = Options {
        template: Template::new("snoop", "#read(\"/Cargo.toml\")"),
        ..Options::default()
    };
    assert!(export::render(&Profile::example(), Format::Pdf, &options).is_err());
}

#[test]
fn docx_is_a_zip() {
    assert!(render(Format::Docx).starts_with(b"PK"));
}

#[test]
fn markdown_has_sections_and_bullets() {
    let md = render_str(Format::Md);
    assert!(md.starts_with("# Alex Morgan\n"));
    assert!(md.contains("## Experience"));
    assert!(md.contains("### Software Engineer — Northwind Robotics"));
    assert!(md.contains("*Apr 2022 – present · Munich, Germany*"));
    assert!(md.contains("- Rewrote the path planner in Rust"));
}

#[test]
fn html_is_standalone_and_escaped() {
    let mut profile = Profile::example();
    profile.basics.name = Some("Alex <script>".into());
    let html =
        String::from_utf8(export::render(&profile, Format::Html, &Options::default()).unwrap())
            .unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("Alex &lt;script&gt;"));
    assert!(!html.contains("<script>"));
}

#[test]
fn text_and_json() {
    assert!(render_str(Format::Txt).starts_with("ALEX MORGAN\n"));
    let json = render_str(Format::Json);
    assert_eq!(Profile::from_json(&json).unwrap(), Profile::example());
}

#[test]
fn builtin_templates_resolve() {
    for name in Template::builtin_names() {
        assert_eq!(Template::resolve(name).unwrap().name(), name);
    }
    assert!(matches!(
        Template::resolve("nope"),
        Err(Error::UnknownTemplate(_))
    ));
}
