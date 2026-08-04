<script lang="ts">
	const features = [
		{ label: 'Post scheduling', desc: 'Threads has no native scheduling. Titen queues and publishes on your clock.' },
		{ label: 'Multi-account', desc: 'Manage multiple Threads profiles from a single instance. Switch without re-auth.' },
		{ label: 'Comments + sentiment', desc: 'Fetch comments and run sentiment analysis. Know how your posts land.' },
		{ label: 'Time-series analytics', desc: 'Snapshot-based analytics that track engagement over time, not just a single point.' },
		{ label: 'S3 media storage', desc: 'Offload media to your own S3 bucket. No vendor lock-in on assets.' },
		{ label: 'MCP server', desc: 'Expose every Titen capability to AI agents via the Model Context Protocol.' },
	];
</script>

<!-- Nav: N9 Edge-aligned minimal -->
<nav class="nav">
	<div class="container nav__inner">
		<a href="/" class="nav__wordmark" aria-label="Titen home">titen</a>
		<div class="nav__actions">
			<a href="/login" class="btn-outline nav__signin">
				Sign in
			</a>
			<a
				href="https://github.com/codecoradev/titen"
				target="_blank"
				rel="noopener noreferrer"
				class="btn-outline nav__cta"
			>
				GitHub ↗
			</a>
		</div>
	</div>
</nav>

<main>
	<!-- Hero: code-first opening — no marketing fluff -->
	<section class="hero">
		<div class="container">
			<h1 class="hero__heading">
				Self-hosted Threads management.<br />
				<span class="hero__heading--muted">One binary. Three interfaces.</span>
			</h1>
			<p class="hero__lede">
				Titen is a Rust + SQLite platform for scheduling posts,
				managing multiple Threads accounts, and analysing engagement —
				all from infrastructure you control.
			</p>
			<div class="hero__actions">
				<a
					href="https://github.com/codecoradev/titen"
					target="_blank"
					rel="noopener noreferrer"
					class="btn-primary"
				>
					View on GitHub
				</a>
				<a href="#install" class="btn-outline">Install</a>
			</div>
			<div class="hero__stack">
				<span><code>Rust</code> core</span>
				<span class="hero__stack-dot">·</span>
				<span><code>SQLite</code> storage</span>
				<span class="hero__stack-dot">·</span>
				<span>Single binary</span>
				<span class="hero__stack-dot">·</span>
				<span><code>AGPL-3.0</code></span>
			</div>
		</div>
	</section>

	<!-- Features grid: asymmetric, not the 3-column template -->
	<section class="features">
		<div class="container">
			<h2>What it does</h2>
			<div class="features__grid">
				{#each features as f, i}
					<div class="feature-card">
						<h3>{f.label}</h3>
						<p>{f.desc}</p>
					</div>
				{/each}
			</div>
		</div>
	</section>

	<!-- API example -->
		<section class="code-section">
			<div class="container">
				<div class="code-section__label">HTTP API</div>
				<h2>Schedule a post</h2>
				<pre><code>curl -X POST http://localhost:7845/api/schedules \
				  -H "X-API-Key: your-key" \
				  -H "Content-Type: application/json" \
				  -d '&#123;
	    "account_id": "...",
	    "caption": "Shipping Titen v0.1 today.",
	    "scheduled_at": "2026-07-23T09:00:00Z"
	  &#125;'</code></pre>
			</div>
		</section>

		<!-- CLI example -->
		<section class="code-section code-section--alt">
			<div class="container">
				<div class="code-section__label">CLI</div>
				<h2>From the terminal</h2>
				<pre><code># List accounts
	titen account list

	# Schedule a post
	titen schedule add codecoradev \
	  --text "Deploying to prod." \
	  --at "2026-07-23T09:00:00+07:00"

	# Fetch and analyze comments
	titen comment fetch &lt;post_id&gt;
	titen comment sentiment &lt;post_id&gt;</code></pre>
			</div>
		</section>

		<!-- Install CTA -->
		<section id="install" class="install">
			<div class="container">
				<div class="install__label">Get started</div>
				<h2 class="install__heading">Install Titen</h2>
				<pre><code># One-line install (Linux / macOS)
	curl -sSL https://github.com/codecoradev/titen/releases/latest/download/install.sh | sh

	# Start the server
	titen serve --port 7845</code></pre>
				<p class="install__note">
					Also available as a <a href="https://github.com/codecoradev/titen/pkgs/container/titen" target="_blank" rel="noopener noreferrer">Docker image</a> and via <code>cargo install titen</code>.
				</p>
			</div>
		</section>
</main>

<!-- Footer: Ft2 Inline rule single line -->
<footer class="footer">
	<div class="container footer__inner">
		<span>titen</span>
		<span class="footer__sep">·</span>
		<span>AGPL-3.0</span>
		<span class="footer__sep">·</span>
		<a href="https://github.com/codecoradev/titen" target="_blank" rel="noopener noreferrer">github.com/codecoradev/titen</a>
	</div>
</footer>

<style>
	/* ── Nav: N9 Edge-aligned minimal ── */
	.nav {
		position: sticky;
		top: 0;
		z-index: var(--z-sticky);
		padding-block: var(--space-md);
		background: oklch(98.5% 0.005 260 / 0.85);
		backdrop-filter: blur(12px);
		-webkit-backdrop-filter: blur(12px);
		border-bottom: 1px solid var(--color-rule);
	}

	.nav__inner {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.nav__actions {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
	}

	.nav__wordmark {
		font-family: var(--font-mono);
		font-weight: 500;
		font-size: var(--text-md);
		color: var(--color-ink);
		letter-spacing: -0.01em;
	}

	.nav__wordmark:hover {
		color: var(--color-ink);
	}

	/* ── Hero ── */
	.hero {
		padding-block: var(--space-3xl) var(--space-2xl);
	}

	.hero__heading {
		max-width: 42rem;
		margin-bottom: var(--space-lg);
	}

	.hero__heading--muted {
		color: var(--color-muted);
		font-weight: 500;
	}

	.hero__lede {
		max-width: 52rem;
		font-size: var(--text-md);
		line-height: 1.6;
		color: var(--color-muted);
		margin-bottom: var(--space-xl);
	}

	.hero__actions {
		display: flex;
		gap: var(--space-sm);
		flex-wrap: wrap;
		margin-bottom: var(--space-xl);
	}

	.hero__stack {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
		flex-wrap: wrap;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-neutral);
	}

	.hero__stack-dot {
		color: var(--color-rule);
	}

	/* ── Features ── */
	.features {
		padding-block: var(--space-2xl);
		border-top: 1px solid var(--color-rule);
	}

	.features h2 {
		margin-bottom: var(--space-xl);
	}

	.features__grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
		gap: var(--space-sm);
	}

	/* Asymmetric spans for visual variety */
	.feature-card {
		padding: var(--space-lg);
		border: 1px solid var(--color-rule);
		border-radius: 8px;
		background: var(--color-paper);
		transition: border-color var(--dur-short) var(--ease-out);
	}

	.feature-card:nth-child(1),
	.feature-card:nth-child(4) {
		grid-column: span 1;
	}

	.feature-card:nth-child(2) {
		grid-column: span 1;
	}

	.feature-card:nth-child(3),
	.feature-card:nth-child(6) {
		grid-column: span 1;
	}

	@media (min-width: 60rem) {
		.features__grid {
			grid-template-columns: 1fr 1fr 1fr;
		}

		/* Break the uniform grid: first card spans 2 */
		.feature-card:nth-child(1) {
			grid-column: span 2;
		}
	}

	.feature-card:hover {
		border-color: var(--color-neutral);
	}

	.feature-card h3 {
		font-size: var(--text-base);
		margin-bottom: var(--space-xs);
	}

	.feature-card p {
		font-size: var(--text-sm);
		color: var(--color-muted);
		line-height: 1.6;
		max-width: 40ch;
	}

	/* ── Code sections ── */
	.code-section {
		padding-block: var(--space-2xl);
		border-top: 1px solid var(--color-rule);
	}

	.code-section--alt {
		background: var(--color-paper-2);
	}

	.code-section__label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-accent);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		margin-bottom: var(--space-xs);
	}

	.code-section h2 {
		margin-bottom: var(--space-lg);
	}

	.code-section pre {
		max-width: 100%;
	}

	/* ── Install ── */
	.install {
		padding-block: var(--space-2xl) var(--space-3xl);
		border-top: 1px solid var(--color-rule);
	}

	.install__label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-accent);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		margin-bottom: var(--space-xs);
	}

	.install__heading {
		margin-bottom: var(--space-lg);
	}

	.install pre {
		max-width: 100%;
		margin-bottom: var(--space-lg);
	}

	.install__note {
		font-size: var(--text-sm);
		color: var(--color-muted);
		max-width: 52rem;
		line-height: 1.6;
	}

	/* ── Footer: Ft2 Inline rule single line ── */
	.footer {
		border-top: 1px solid var(--color-rule);
		padding-block: var(--space-lg);
	}

	.footer__inner {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
		flex-wrap: wrap;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-neutral);
	}

	.footer__sep {
		color: var(--color-rule);
	}

	.footer a {
		color: var(--color-muted);
	}

	.footer a:hover {
		color: var(--color-ink);
	}
</style>