# Anna's Kazib

Anna's Kazib is a web-based download manager application for [Anna's Archive](https://annas-archive.org/). It provides a user-friendly interface to search for and download books from Anna's Archive, with features like filtering, library management, and download history.

## Features

- **Search Books**: Search for books across Anna's Archive with powerful filtering options
- **Filter System**: Filter by language, file format (EPUB, PDF, etc.), and content type
- **Library Management**: Track which books you've already downloaded
- **Download History**: View your download history with status tracking
- **Responsive Design**: Works on desktop and mobile devices
- **Cross-platform**: Web-based interface that runs in your browser

## Running locally

### Prerequisites

**Install Rust**

```shell
rustup toolchain install stable
rustup target add wasm32-unknown-unknown
```

**Install the Dioxus CLI**

```shell
cargo binstall dioxus-cli --version 0.7.4 --force
```

Depending on your OS you may need additional system dependencies, please refer to the [dioxus getting started guide](https://dioxuslabs.com/learn/0.7/getting_started).

### Run the app

```shell
dx serve --package kazib --web
```

This will start the development server. Open your browser and navigate to `http://localhost:8080`.

## Build for release

To build an optimized production build:

```shell
dx build --release --package kazib --web
```

The output will be in the `dist/` directory, ready to be served by any static file server.

## Configuration

The application uses a SQLite database (`kazib.db`) to store:
- Download history
- Library entries
- User settings

The database is automatically created in the project root directory on first run.

### Library Path Template

**One of the most powerful features of Kazib is the customizable library path template system.** This allows you to define exactly where and how your downloaded books are saved, using metadata variables and smart operators.

#### Available Variables

| Variable | Description | Availability |
|----------|-------------|--------------|
| `{{title}}` | Book title | Always |
| `{{author}}` | Author name | Optional |
| `{{series}}` | Series name | Optional |
| `{{series_number}}` | Position in series | Optional |
| `{{language}}` | Language code (en, fr, etc.) | Optional |
| `{{year}}` | Publication year | Optional |
| `{{ext}}` | File extension (epub, pdf, etc.) | Optional |

#### Template Operators

| Syntax | Description | Example |
|--------|-------------|---------|
| `{{name}}` | Required variable (download fails if missing) | `{{author}}` → Tolkien |
| `{{name:default}}` | Fallback value if variable is missing | `{{series:Standalone}}` → Standalone |
| `{{name/}}` | Optional path segment (skipped if missing, adds `/`) | `{{language/}}` → `en/` or nothing |
| `{{name:default/}}` | Fallback + optional path segment | `{{series:_oneshots/}}` |
| `{{?name}}...{{/name}}` | Conditional block (only rendered if variable exists) | `{{?series}}{{series}} #{{series_number}} - {{/series}}` |

#### Examples

- **Simple**: `/books/{{author}}/{{title}}.{{ext}}`
- **With optional language**: `/books/{{language/}}{{author}}/{{title}}.{{ext}}`
- **With series prefix**: `/books/{{author}}/{{?series}}{{series}} - {{series_number}} - {{/series}}{{title}}.{{ext}}`
- **Complete**: `/ebooks/{{language}}/{{author}}/{{series:_oneshots}}/{{?series}}{{series}} - {{series_number}} - {{/series}}{{title}}.{{ext}}`

## Docker

You can also run the application using Docker:

```shell
docker build -t kazib .
docker run -p 3000:8080 kazib -v ./data:/app/data --user me:users
```

## Acknowledgment

Most of the Anna's Archive API code has been taken from [RemiKalbe/annas-archive-mcp](https://github.com/remikalbe/annas-archive-mcp)

## Gallery

| ![Search Page](docs/screenshots/search_screenshot.png) | ![Settings Page](docs/screenshots/settings_screenshot.png) |
|:-------------------------------------------------------:|:--------------------------------------------------------:|
| ![History Page](docs/screenshots/history_screenshot.png) |                                                        |

## LICENSE

This project is licensed under the GNU General Public License v3.0. For full details, see the [LICENSE](LICENSE) file.
