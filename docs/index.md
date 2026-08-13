---
layout: default
title: sc-compose
description: Compose once. Render deterministically. Ship everywhere.
---

<section class="hero">
  <h1>sc-compose</h1>
  <p class="tagline">Compose once. Render deterministically. Ship everywhere.</p>
  <p class="subtitle">
    A standalone CLI and composable library for teams whose templates have
    outgrown copy-paste. Compose agent profiles, config files, test fixtures,
    and reports from shared fragments — with declared inputs that fail loudly
    when missing.
  </p>
  <div class="cta">
    <a href="{{ '/install' | relative_url }}" class="btn btn-primary">Get Started</a>
    <a href="https://github.com/randlee/sc-compose" class="btn btn-secondary">GitHub</a>
  </div>
</section>

<section id="features">
  <h2>Why sc-compose?</h2>

  <div class="feature-grid">
    <div class="feature">
      <h3>🔗 Compose from Shared Fragments</h3>
      <p>
        Place your house style, review checklist, and error conventions in one
        includable file. Reference it from every template with <code>@<_includes/house-style.md></code>.
        Edit once — every downstream template picks up the change.
      </p>
    </div>

    <div class="feature">
      <h3>✅ Declared Inputs, Loud Failures</h3>
      <p>
        Declare required variables in YAML frontmatter. Missing a
        <code>task_id</code>? sc-compose fails with an actionable diagnostic
        that names the variable, the file that declared it, and the include
        chain. No silent guessing.
      </p>
    </div>

    <div class="feature">
      <h3>🔀 One Profile, Four Runtimes</h3>
      <p>
        Author an agent profile once under <code>.agents/agents/</code>. It
        resolves correctly for Claude Code, Codex, Gemini, and OpenCode through
        each runtime's native search chain. Override only the runtimes that
        genuinely need specialization.
      </p>
    </div>

    <div class="feature">
      <h3>🔄 Multi-Pass Nested Templates</h3>
      <p>
        Define deploy-time, install-time, and invocation-time variables in one
        file. Stacked YAML headers with progressive brace counts. Render all
        passes in one command. Verify deployed output hasn't drifted from the
        template source.
      </p>
    </div>

    <div class="feature">
      <h3>🐍 Python Bindings</h3>
      <p>
        <code>pip install sc-compose</code>. Native extension module built with
        PyO3 and maturin. Pre-built wheels for macOS, Linux, and Windows
        (Python 3.11+). Full multi-pass rendering from Python.
      </p>
    </div>

    <div class="feature">
      <h3>📊 Built-in Reporting</h3>
      <p>
        Produce compliance evidence from declarative semantic specs. Scaffold a
        report catalog, render HTML reports, materialize metadata, and generate
        CI handoff manifests — all from the CLI.
      </p>
    </div>
  </div>
</section>

<section id="quickstart">
  <h2>Quickstart</h2>

  <div class="install-options">
    <div class="install-card">
      <h4>macOS</h4>
      <code>brew install randlee/tap/sc-compose</code>
    </div>
    <div class="install-card">
      <h4>Windows</h4>
      <code>winget install randlee.sc-compose</code>
    </div>
    <div class="install-card">
      <h4>Rust</h4>
      <code>cargo install sc-compose</code>
    </div>
    <div class="install-card">
      <h4>Python</h4>
      <code>pip install sc-compose</code>
    </div>
  </div>

  <div class="code-example">
{% highlight bash %}
# Render your first template
echo 'Hello {{ "{{ name }}" }}!' > hello.txt.j2
sc-compose render --file hello.txt.j2 --var name=World
# → Hello World!

# Compose a profile for Claude Code
sc-compose render --mode profile --kind agent \
  --agent rust-developer --runtime claude

# Generate pytest stubs from a bundled example
sc-compose examples pytest-fixture \
  --var-file tests.json --output tests/test_auth.py
{% endhighlight %}
  </div>
</section>

<section id="install-matrix">
  <h2>Install Everywhere</h2>

  <table>
    <thead>
      <tr><th>Platform</th><th>Method</th><th>Command</th></tr>
    </thead>
    <tbody>
      <tr><td>macOS</td><td>Homebrew</td><td><code>brew install randlee/tap/sc-compose</code></td></tr>
      <tr><td>Windows</td><td>Winget</td><td><code>winget install randlee.sc-compose</code></td></tr>
      <tr><td>Any (Rust)</td><td>crates.io</td><td><code>cargo install sc-compose</code></td></tr>
      <tr><td>Any (Python)</td><td>PyPI</td><td><code>pip install sc-compose</code></td></tr>
      <tr><td>Any (source)</td><td>cargo</td><td><code>cargo build --release -p sc-compose</code></td></tr>
      <tr><td>Rust lib</td><td>Cargo.toml</td><td><code>sc-composer = "1.4.1"</code></td></tr>
    </tbody>
  </table>

  <p class="note">
    Bundled examples are guaranteed in Homebrew, Winget, and GitHub Release
    installs. <code>cargo install</code> ships the binary only — set
    <code>SC_COMPOSE_DATA_DIR</code> for examples.
  </p>
</section>

<section id="use-cases">
  <h2>What People Use It For</h2>

  <div class="use-case-grid">
    <div class="use-case">
      <h4>AI Agent Profiles</h4>
      <p>Author once, resolve across Claude, Codex, Gemini, and OpenCode.</p>
    </div>
    <div class="use-case">
      <h4>Task Templates</h4>
      <p>Generate structured XML/JSON task assignments with declared inputs.</p>
    </div>
    <div class="use-case">
      <h4>pytest Fixtures</h4>
      <p>Generate test stubs from a list of test names with bundled examples.</p>
    </div>
    <div class="use-case">
      <h4>Service Configs</h4>
      <p>Compose YAML configs from shared fragments with env-var inputs.</p>
    </div>
    <div class="use-case">
      <h4>Sprint Reports</h4>
      <p>Generate HTML compliance reports from declarative semantic specs.</p>
    </div>
    <div class="use-case">
      <h4>.NET Benchmarks</h4>
      <p>Compose benchmark harnesses from shared setup and teardown fragments.</p>
    </div>
  </div>
</section>

<section id="docs">
  <h2>Documentation</h2>

  <ul>
    <li><a href="https://github.com/randlee/sc-compose/blob/develop/docs/requirements.md">Requirements</a> — normative behavior, JSON schemas, exit codes</li>
    <li><a href="https://github.com/randlee/sc-compose/blob/develop/docs/architecture.md">Architecture</a> — library module layout and crate boundaries</li>
    <li><a href="https://github.com/randlee/sc-compose/blob/develop/docs/error-code-registry.md">Error Codes</a> — stable <code>ERR_*</code> diagnostic codes</li>
    <li><a href="https://github.com/randlee/sc-compose/blob/develop/RELEASING.md">Releasing</a> — step-by-step release checklist</li>
    <li><a href="https://github.com/randlee/sc-compose/blob/develop/docs/publishing.md">Publishing</a> — distribution channels and secrets</li>
    <li><a href="{{ '/examples' | relative_url }}">Examples</a> — bundled starter templates</li>
  </ul>
</section>

<section id="status">
  <h2>Status</h2>

  <table>
    <tr><td>Version</td><td><strong>1.4.1</strong></td></tr>
    <tr><td>MSRV</td><td>Rust 1.94.1</td></tr>
    <tr><td>Edition</td><td>2024</td></tr>
    <tr><td>Platforms</td><td>macOS, Linux, Windows</td></tr>
    <tr><td>Stability</td><td>stable 1.x release line</td></tr>
  </table>
</section>
