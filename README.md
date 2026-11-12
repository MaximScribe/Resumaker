# Resumaker

A lightweight CLI that writes a resume tailored to a specific job, plus an optional cover letter. It merges your experience from local files and APIs, and runs on a local model by default.

```bash
resumaker                                                  # one-time setup
resumake -c "Backend developer, TypeScript + Postgres" -m  # tailored resume + cover letter
```

> **Status:** early development. The interface below is the planned design. See the [roadmap](#roadmap).

## Features

- **Tailored to the job.** Describe the role as text, or pass a job-posting file or URL.
- **Cover letters.** Add one with `-m`. You can give it notes or files of its own.
- **Many sources.** A folder of `.md`, `.txt`, `.pdf`, `.docx`, `.xlsx`, `.csv` or `.json` files, and/or any HTTP API.
- **Your own layout.** Give it a PDF of a resume you like and Resumaker builds a matching template. Or use a preset.
- **Multiple formats.** Export PDF, DOCX, Markdown, HTML, TXT and JSON.
- **Feedback loop.** Review a result, ask for changes, and Resumaker remembers your preferences in later runs.
- **No invented facts.** Every claim must come from one of your sources. A check pass flags anything that doesn't.
- **Private by default.** Built for local inference. Cloud providers are optional.

## Privacy

Resumes contain personal data. **Local inference is recommended**, so nothing leaves your machine.

| Local (recommended) | Cloud (optional) |
|---|---|
| Ollama, LM Studio, llama.cpp, any OpenAI-compatible server | Gemini, OpenRouter, OpenAI, Anthropic |

If you choose a cloud provider, setup tells you that your source files will be sent to it. Cloud API keys are stored in your OS keychain, never in plain-text config.

## Installation

```bash
npm install -g resumaker      # installs the `resumaker` and `resumake` commands
resumaker                     # run setup
```

Requires Node.js 20 or newer. PDF rendering uses [Typst](https://typst.app), which is bundled.

## Setup: `resumaker`

Running `resumaker` starts an interactive wizard. Use the arrow keys and Enter to choose, and Space to toggle options. Once you're set up, running it again opens a settings menu.

```
  ____                                 _
 |  _ \ ___  ___ _   _ _ __ ___   __ _| | _____ _ __
 | |_) / _ \/ __| | | | '_ ` _ \ / _` | |/ / _ \ '__|
 |  _ <  __/\__ \ |_| | | | | | | (_| |   <  __/ |
 |_| \_\___||___/\__,_|_| |_| |_|\__,_|_|\_\___|_|

 ? Which model should write your resumes?
 ❯ Ollama (local, recommended)
   LM Studio (local)
   Other local server (OpenAI-compatible)
   Gemini
   OpenRouter
   OpenAI
   Anthropic
```

1. **Inference provider:** choose a provider and model. For cloud providers you also enter an API key.
2. **Experience sources:** a folder of files, an API, or both.
3. **Resume style:** a reference PDF of your own, or a preset template.
4. **Output folder:** where resumes and cover letters are saved.
5. **Export formats:** choose one or more with Space.

You can also change settings directly:

| Command | Description |
|---|---|
| `resumaker set provider <name> [--model <id>]` | Change the inference provider |
| `resumaker set source folder <path>` | Set the folder of experience files |
| `resumaker set source api <url> [--token <t>]` | Add an API source |
| `resumaker set style <pdf \| preset>` | Change the reference resume or preset |
| `resumaker set output <path>` | Change the output folder |
| `resumaker set formats <list>` | Change the default formats, e.g. `pdf,docx` |
| `resumaker auth add \| list \| remove \| test` | Manage cloud API keys |
| `resumaker sources [pull]` | List sources, or fetch and print the merged profile |
| `resumaker templates list \| download \| preview <name>` | Manage templates |
| `resumaker feedback list \| clear` | View or delete saved feedback |

## Usage: `resumake`

```bash
resumake [options]
```

| Option | Short | Description |
|---|---|---|
| `--context <text\|file\|url>` | `-c` | The job to tailor to. Repeatable |
| `--cover-letter` | `-m` | Also write a cover letter |
| `--cover-context <text\|file>` | `-C` | Notes or files for the cover letter only. Repeatable; implies `-m` |
| `--feedback` | `-F` | Review and revise interactively, then save your feedback |
| `--file <path>` | `-f` | Extra experience files for this run. Repeatable, globs allowed |
| `--out <dir>` | `-o` | Output folder |
| `--format <list>` | | Export formats for this run |
| `--style <preset\|pdf>` | `-s` | Template for this run |
| `--lang <code>` | `-l` | Output language, e.g. `en`, `de` |
| `--pages <1\|2>` | | Maximum resume length (default `1`) |
| `--cover-only` | | Write only the cover letter |
| `--no-sources` | | Ignore configured sources and use only `--file` |
| `--provider <name>`, `--model <id>` | | Override the model for this run |
| `--dry-run` | | Show the sources and plan without calling the model |
| `--force` | `-y` | Overwrite existing files |
| `--verbose` | `-v` | Show each pipeline step |

If you give no `--context`, you get a general-purpose resume. With `-m`, you also get a generic cover letter. Use `-C` for things only the letter should mention, such as why you want this company or when you can start.

```bash
# General resume and generic cover letter
resumake -m

# Tailored to a job posting, with notes for the cover letter
resumake -c ./posting.pdf -m -C "Available from March, relocating to Munich"

# Job posting URL, exported as DOCX, with a feedback loop
resumake -c https://example.com/jobs/123 --format docx -F
```

### Feedback loop

With `-F`, Resumaker asks for feedback after each version, until you submit an empty line:

```
✔ Resume written → ~/Applications/2026-11-12_example-corp/resume.pdf
? How did it turn out? (empty to finish)
> Put the backend projects first and shorten the summary
✔ Resume updated
? How did it turn out? (empty to finish)
>
✔ Feedback saved → <sources>/resumaker-feedback/2026-11-12_example-corp.txt
```

Feedback is saved as plain text in your sources folder, or in the config folder if you have no sources folder. Later runs read it as **style guidance** (order, tone, emphasis). It is never treated as experience.

## Connecting an API

Resumaker works with any HTTP endpoint and tries three ways to read it:

1. **[JSON Resume](https://jsonresume.org/schema):** if the endpoint returns this open standard, Resumaker reads it directly. This is the recommended format for your own backend.
2. **Adapters:** small mapping modules for known APIs, in `src/sources/adapters/`. Contributions welcome.
3. **Any JSON:** anything else is treated like a source file, and the model reads it as part of merging.

Bearer tokens and custom headers are supported. You can connect several APIs at once.

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

A reference PDF is turned into a Typst template during setup. Resumaker extracts the layout, section order, fonts and colours, and you confirm a preview before it is saved. The result closely matches the original but is not a pixel-perfect copy.

## Configuration

Settings are stored in `~/.config/resumaker/config.toml` (`%APPDATA%\resumaker\` on Windows).

```toml
[inference]
provider = "ollama"
model    = "qwen3:14b"

[sources]
folder = "~/Documents/Career"
apis   = [{ url = "https://api.example.com/resume" }]

[output]
folder  = "~/Documents/Applications"
formats = ["pdf", "md"]
style   = "my-resume"
lang    = "en"
```

Environment variables override keychain keys, for example `RESUMAKER_OPENAI_KEY`.

## Roadmap

- [ ] v0.1: profile schema and PDF export from example data
- [ ] v0.2: setup wizard, inference providers, folder sources
- [ ] v0.3: tailored resumes, all export formats
- [ ] v0.4: cover letters, feedback loop
- [ ] v0.5: API sources
- [ ] v0.6: reference PDF templates, preset template pack
- [ ] v1.0: enforced fact check, stable release
- [ ] Optional GUI

## License

[MIT](LICENSE)
