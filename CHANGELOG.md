# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [1.0.0] - 2026-11-23

First stable release, with prebuilt binaries.

### Added
- Fact check for generated resumes and cover letters: employers, schools, certificates, dates, numbers and skills are compared against the sources; the model gets one chance to correct unsupported claims, and `--strict` turns remaining findings into an error.
- Release workflow building Windows, Linux and macOS (Apple Silicon and Intel) binaries with checksums.

### Changed
- Release builds use thin link-time optimisation, which needs far less memory to compile.
- README rewritten as user documentation.

## [0.6.0] - 2026-11-23

### Added
- Templates generated from a reference resume PDF (`resumaker set style my-cv.pdf` or in the wizard): fonts and accent colour are read from the PDF, the layout is inferred by the model, with a preview.
- Downloadable template pack (`resumaker templates download`): modern, compact, academic and lebenslauf.

## [0.5.0] - 2026-11-23

### Added
- API sources: JSON Resume endpoints are used directly, adapters map known APIs (GitHub repositories), anything else is read by the model. Bearer tokens are kept in the keychain.
- `resumaker sources pull` prints the merged profile as JSON.

## [0.4.0] - 2026-11-22

### Added
- Cover letters with `-m`, letter-only notes with `-C` and `--cover-only`.
- Feedback loop with `-F`; saved feedback guides later runs as style preferences.

## [0.3.1] - 2026-11-22

### Fixed
- Clippy warning in glob expansion.

## [0.3.0] - 2026-11-22

### Added
- `resumake`: merges sources, tailors the resume to a job (text, file or URL) and exports it.

## [0.2.0] - 2026-11-22

### Added
- Interactive setup wizard, settings commands and keychain-backed API keys.
- Ollama, LM Studio, OpenAI-compatible, Gemini, OpenRouter, OpenAI and Anthropic providers.
- Reading experience from folders of text, PDF, DOCX and spreadsheet files.

## [0.1.0] - 2026-11-22

### Added
- JSON Resume-based profile schema with source tracking.
- Export to PDF (embedded Typst), DOCX, Markdown, HTML, text and JSON.

[1.0.0]: https://github.com/MaximScribe/Resumaker/releases/tag/v1.0.0
[0.6.0]: https://github.com/MaximScribe/Resumaker/releases/tag/v0.6.0
[0.5.0]: https://github.com/MaximScribe/Resumaker/releases/tag/v0.5.0
[0.4.0]: https://github.com/MaximScribe/Resumaker/releases/tag/v0.4.0
[0.3.1]: https://github.com/MaximScribe/Resumaker/releases/tag/v0.3.1
[0.3.0]: https://github.com/MaximScribe/Resumaker/releases/tag/v0.3.0
[0.2.0]: https://github.com/MaximScribe/Resumaker/releases/tag/v0.2.0
[0.1.0]: https://github.com/MaximScribe/Resumaker/releases/tag/v0.1.0
