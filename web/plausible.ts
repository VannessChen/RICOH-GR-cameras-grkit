import { createHash } from 'node:crypto';

/** The Plausible script, with the extensions that the site uses. */
const SCRIPT =
  '/js/script.file-downloads.hash.outbound-links.pageview-props.revenue.tagged-events.js';
/** The site in Plausible. */
const DOMAIN = 'grkit.app';
/** Queues the events that the page sends before the script loads. */
const QUEUE =
  'window.plausible = window.plausible || function() { (window.plausible.q = window.plausible.q || []).push(arguments) }';

/** Returns `host` as an https origin without a path, or throws. */
export function plausibleOrigin(host: string): string {
  const url = new URL(host);
  if (url.protocol !== 'https:' || url.origin !== host.replace(/\/$/, '')) {
    throw new Error(`PLAUSIBLE_HOST must be an https origin without a path: ${host}`);
  }
  return url.origin;
}

/** Adds the Plausible script and its event queue to the end of the head of `html`. */
export function addPlausible(html: string, origin: string): string {
  const tags =
    `<script defer data-domain="${DOMAIN}" src="${origin}${SCRIPT}"></script>\n` +
    `  <script>${QUEUE}</script>\n  `;
  if (!html.includes('</head>')) throw new Error('index.html has no </head>');
  return html.replace('</head>', `${tags}</head>`);
}

/**
 * Lets the Content-Security-Policy of `_headers` load the script from `origin`, run the queue
 * (by its hash, without 'unsafe-inline'), and send events to `origin`.
 */
export function allowPlausible(headers: string, origin: string): string {
  const hash = `'sha256-${createHash('sha256').update(QUEUE).digest('base64')}'`;
  let policies = 0;
  const policy = /^(\s*Content-Security-Policy: )(.*)$/m;
  const result = headers.replace(policy, (_, name: string, value: string) => {
    policies += 1;
    const directives = value.split(';').map((directive) => directive.trim());
    const extend = (directive: string, sources: string[]) => {
      const index = directives.findIndex((item) => item.split(' ')[0] === directive);
      if (index < 0) throw new Error(`The Content-Security-Policy has no ${directive}`);
      directives[index] = [directives[index], ...sources].join(' ');
    };
    extend('script-src', [origin, hash]);
    extend('connect-src', [origin]);
    return name + directives.join('; ');
  });
  if (policies !== 1) throw new Error('_headers must have one Content-Security-Policy');
  return result;
}
