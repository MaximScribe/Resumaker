use clap::ValueEnum;

/// File formats a resume can be exported as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Pdf,
    Docx,
    Md,
    Html,
    Txt,
    Json,
}
