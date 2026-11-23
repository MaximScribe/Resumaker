# Resumaker

A lightweight CLI that writes a resume tailored to a specific job, plus an optional cover letter. It merges your experience from local files and APIs, and runs on a local model by default.

```bash
resumaker                                                  # one-time setup
resumake -c "Backend developer, TypeScript + Postgres" -m  # tailored resume + cover letter
```

## Features

- **Tailored to the job.** Describe the role as text, or pass a job-posting file or URL.
- **Cover letters.** Add one with `-m`. You can give it notes or files of its own.
- **Many sources.** A folder of `.md`, `.txt`, `.pdf`, `.docx`, `.xlsx`, `.csv` or `.json` files, and/or any HTTP API.
- **Your own layout.** Give it a PDF of a resume you like and Resumaker builds a matching template. Or use a preset.
- **Multiple formats.** Export PDF, DOCX, Markdown, HTML, TXT and JSON.
- **Feedback loop.** Review a result, ask for changes, and Resumaker remembers your preferences in later runs.
- **No invented facts.** A built-in fact check compares employers, schools, dates, numbers and skills against your sources and asks the model to fix anything unsupported.
- **Private by default.** Built for local inference. Cloud providers are optional.

## Privacy

Resumes contain personal data. **Local inference is recommended**, so nothing leaves your machine.

| Local (recommended) | Cloud (optional) |
|---|---|
| Ollama, LM Studio, llama.cpp, any OpenAI-compatible server | Gemini, OpenRouter, OpenAI, Anthropic |

If you choose a cloud provider, setup tells you that your source files will be sent to it. Cloud API keys are stored in your OS keychain, never in plain-text config.

## Installation

Resumaker is a single self-contained binary with no runtime to install. PDF rendering ([Typst](https://typst.app)) is built in.

**Install script (recommended).** It downloads the latest release, verifies its checksum, installs `resumaker` and `resumake` and puts them on your `PATH`, so both commands work in any new terminal.

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/MaximScribe/Resumaker/main/install.ps1 | iex
```

Linux and macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/MaximScribe/Resumaker/main/install.sh | sh
```

The binaries go to `%LOCALAPPDATA%\Programs\Resumaker` on Windows and `~/.local/bin` on Linux and macOS. Run the same command again to update. Set `RESUMAKER_VERSION=v1.0.1` to install a specific release, or `RESUMAKER_INSTALL_DIR` to choose the folder.

To uninstall, delete the two binaries from that folder. On Windows, also remove the folder from your user `PATH` (Settings → System → About → Advanced system settings → Environment Variables). On Linux and macOS, remove the line marked `# Added by the Resumaker installer` from your shell startup file. Your settings stay in the config directory (see [Configuration](#configuration)).

**Manual download:** download the archive for your platform from the [Releases](https://github.com/MaximScribe/Resumaker/releases) page, unpack it and put `resumaker` and `resumake` on your `PATH`. Each archive has a `.sha256` checksum next to it. The Linux binary needs glibc 2.35 or newer (Ubuntu 22.04, Debian 12, RHEL 9, Fedora 36 or later).

**From source** (requires [Rust](https://rustup.rs) 1.92+):

```bash
cargo install --git https://github.com/MaximScribe/Resumaker resumaker-cli   # installs `resumaker` and `resumake`
```

Building from source on Windows requires the [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the C++ workload. On Linux you need a C toolchain (`gcc`); on macOS, the Xcode Command Line Tools.

| Platform | Binaries |
|---|---|
| Windows (x86_64) | ✔ |
| Linux (x86_64) | ✔ |
| macOS (Apple Silicon, Intel) | ✔ |

## Quick start

1. Install a local model runner, for example [Ollama](https://ollama.com), and pull a model: `ollama pull qwen3:14b`. Smaller models work too, but tailoring quality improves with size.
2. Put your experience in a folder: old CVs, certificates, project notes, spreadsheets.
3. Run `resumaker` and follow the setup.
4. Run `resumake -c "<job description or posting URL>" -m`.

Your documents land in the output folder, one folder per application: `<output>/<date>_<company>/resume.pdf` and `cover-letter.pdf`.

## Setup: `resumaker`

Running `resumaker` starts an interactive wizard. Use the arrow keys and Enter to choose, and Space to toggle options. Once you're set up, running it again opens a settings menu.

```
  ____                                 _
 |  _ \ ___  ___ _   _ _ __ ___   __ _| | _____ _ __
 | |_) / _ \/ __| | | | '_ ` _ \ / _` | |/ / _ \ '__|
 |  _ <  __/\__ \ |_| | | | | | | (_| |   <  __/ |
 |_| \_\___||___/\__,_|_| |_| |_|\__,_|_|\_\___|_|

 [1/5] Inference provider
? Which model provider should write your resumes?
> Ollama (local, recommended)
  LM Studio (local)
  OpenAI-compatible server (local)
  Google Gemini (cloud)
  OpenRouter (cloud)
  OpenAI (cloud)
  Anthropic (cloud)
```

1. **Inference provider:** choose a provider, then a model from the list it reports. Cloud providers also ask for an API key. An optional test prompt checks that everything works.
2. **Experience sources:** a folder of files, an API, or both.
3. **Resume style:** a built-in or downloaded template, your own `.typ` template, or a resume PDF whose look you like.
4. **Output folder:** where resumes and cover letters are saved.
5. **Export formats:** choose one or more with Space. Setup finishes with the output language.

You can also change settings directly:

| Command | Description |
|---|---|
| `resumaker set provider <name> [--model <id>] [--url <url>]` | Change the inference provider |
| `resumaker set source folder <path>` | Set the folder of experience files |
| `resumaker set source api <url> [--adapter <name>] [--token]` | Add an API source; `--token` asks for a bearer token |
| `resumaker set source remove-api <url>` | Remove an API source |
| `resumaker set style <name \| template.typ \| resume.pdf>` | Change the template, or generate one from a resume PDF |
| `resumaker set output <path>` | Change the output folder |
| `resumaker set formats <list>` | Change the default formats, e.g. `pdf,docx` |
| `resumaker set lang <code>` | Change the output language, e.g. `en`, `de` |
| `resumaker set pages <1\|2>` | Change the maximum resume length |
| `resumaker auth add \| list \| remove \| test` | Manage cloud API keys and test the connection |
| `resumaker sources [pull]` | List sources, or fetch and print the merged profile as JSON |
| `resumaker templates list \| download \| preview <name>` | Manage templates |
| `resumaker feedback list \| clear` | View or delete saved feedback |
| `resumaker config` | Show the config file location and contents |

## Usage: `resumake`

```bash
resumake [options]
```

| Option | Short | Description |
|---|---|---|
| `--context <text\|file\|url>` | `-c` | The job to tailor to. Repeatable |
| `--cover-letter` | `-m` | Also write a cover letter |
| `--cover-context <text\|file>` | `-C` | Notes or files for the cover letter only. Repeatable; implies `-m` |
| `--cover-only` | | Write only the cover letter |
| `--feedback` | `-F` | Review and revise interactively, then save your feedback |
| `--file <path>` | `-f` | Extra experience files for this run. Repeatable, globs allowed |
| `--no-sources` | | Ignore configured sources and use only `--file` |
| `--out <dir>` | `-o` | Output folder |
| `--format <list>` | | Export formats for this run |
| `--style <name\|file.typ>` | `-s` | Template for this run |
| `--lang <code>` | `-l` | Output language, e.g. `en`, `de` |
| `--pages <1\|2>` | | Maximum resume length |
| `--force` | `-y` | Reuse today's application folder instead of creating a new one |
| `--provider <name>`, `--model <id>` | | Override the model for this run |
| `--strict` | | Fail instead of warning if the fact check still finds unsupported claims |
| `--dry-run` | | Show the sources and plan without calling the model |
| `--verbose` | `-v` | Show more detail |

Options without a value on the command line fall back to your settings. If you give no `--context`, you get a general-purpose resume, and with `-m` a generic cover letter. Use `-C` for things only the letter should mention, such as why you want this company or when you can start.

```bash
# General resume and generic cover letter
resumake -m

# Tailored to a job posting, with notes for the cover letter
resumake -c ./posting.pdf -m -C "Available from March, relocating to Munich"

# Job posting URL, in German, exported as DOCX, with a feedback loop
resumake -c https://example.com/jobs/123 -l de --format docx -F
```

### Feedback loop

With `-F`, Resumaker asks for feedback after each version and regenerates into the same folder, until you press Enter on an empty line:

```
saved in ~/Documents/Resumaker/2026-11-23_example-corp
  ✔ resume.pdf
? How did it turn out? Put the backend projects first and shorten the summary
tailoring the resume…
saved in ~/Documents/Resumaker/2026-11-23_example-corp
  ✔ resume.pdf
? How did it turn out?
feedback saved to ~/Career/resumaker-feedback/2026-11-23_example-corp.txt; future runs will take it into account
```

Feedback is saved as plain text in your sources folder, or in the config folder if you have no sources folder. Later runs read it as **style guidance** (order, tone, emphasis). It is never treated as experience. You can edit or delete the files by hand.

### Fact check

Every resume and cover letter is checked against your sources before it is saved:

- every entry must cite one of your sources;
- employers, schools, certificates and dates must appear in your sources;
- every number (percentages, team sizes, revenue) must appear in your sources;
- every skill must appear in your sources.

If something fails, the model gets one chance to correct it. Anything left is printed as a warning; with `--strict` the run fails instead. When writing in another language than your sources, translated skill names may be flagged; check them and decide.

## Templates

`classic` is built in. `resumaker templates download` adds `modern`, `compact`, `academic` and `lebenslauf` (a tabular German CV). Preview any of them with `resumaker templates preview <name>`.

**From a resume you like:** `resumaker set style my-cv.pdf` (or the wizard) reads the fonts and accent colour from the PDF and lets the model infer the layout: sidebar or single column, heading style, date column, density and section order. You get a preview first, and the generated `.typ` file starts with plain settings you can edit. The result closely matches the original but is not a pixel-perfect copy.

**Your own:** templates are [Typst](https://typst.app) files. They read the prepared resume from `/view.json` (plus `/profile.json` and `/options.json`); see [`classic.typ`](crates/resumaker-core/assets/templates/classic.typ) for an example.

## Connecting an API

Resumaker works with any HTTP endpoint that returns JSON and tries three ways to read it:

1. **[JSON Resume](https://jsonresume.org/schema):** if the endpoint returns this open standard, Resumaker uses it directly, without the model. This is the recommended format for your own backend.
2. **Adapters:** small mapping modules for known APIs, in [`sources/adapters`](crates/resumaker-core/src/sources/adapters). Built in: `github`, which turns `https://api.github.com/users/<name>/repos` into projects.
3. **Any JSON:** anything else is treated like a source file, and the model reads it as part of merging.

Force a mode with `--adapter json-resume`, `--adapter raw` or an adapter name. Bearer tokens are stored in the keychain, and you can connect several APIs at once. `resumaker sources pull > profile.json` shows exactly what Resumaker extracted.

## How it works

```
 files ─┐
 APIs ──┼─► parse ─► merge ─► profile (facts with sources)
        │                        │
 job context + feedback ─► tailor ◄┘
                            │
                        fact check
                            │
                  template ─► export (pdf, docx, md, …)
                            └► cover letter
```

Merged profiles are cached, so unchanged sources are not re-read by the model on every run.

## Configuration

Settings are stored in `config.toml` in your config directory: `~/.config/resumaker/` on Linux, `%APPDATA%\resumaker\` on Windows, `~/Library/Application Support/resumaker/` on macOS. Set `RESUMAKER_CONFIG` to use another file. Downloaded templates and the profile cache live next to it.

```toml
[inference]
provider = "ollama"
model = "qwen3:14b"

[sources]
folder = "~/Documents/Career"

[[sources.apis]]
url = "https://api.github.com/users/octocat/repos"

[output]
folder = "~/Documents/Applications"
formats = ["pdf", "md"]
style = "classic"
lang = "en"
pages = 1
```

API keys never go in this file. Environment variables override keychain keys, for example `RESUMAKER_OPENAI_KEY`, which is useful on systems without a keychain.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

[MIT](LICENSE)
