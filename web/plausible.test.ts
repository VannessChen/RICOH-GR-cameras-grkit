import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { addPlausible, allowPlausible, plausibleOrigin } from './plausible';

const origin = 'https://plausible.example.com';

describe('Plausible', () => {
  it('accepts only an https origin', () => {
    expect(plausibleOrigin('https://plausible.example.com/')).toBe(origin);
    expect(() => plausibleOrigin('http://plausible.example.com')).toThrow();
    expect(() => plausibleOrigin('https://plausible.example.com/js')).toThrow();
    expect(() => plausibleOrigin('plausible.example.com')).toThrow();
  });

  it('adds the script and the queue to the head', () => {
    const html = addPlausible('<html><head><title>GRKit</title></head><body></body></html>', origin);
    expect(html).toContain(
      `<script defer data-domain="grkit.app" src="${origin}/js/script.file-downloads.hash.outbound-links.pageview-props.revenue.tagged-events.js"></script>`,
    );
    expect(html.indexOf('window.plausible')).toBeLessThan(html.indexOf('</head>'));
  });

  it('lets the site policy load the script, run the queue, and send events', () => {
    const site = readFileSync(new URL('public/_headers', import.meta.url), 'utf8');
    const headers = allowPlausible(site, origin);
    const policy = /Content-Security-Policy: (.*)/.exec(headers)![1]!;
    const directive = (name: string) => policy.split('; ').find((item) => item.startsWith(`${name} `));
    const queue = /<script>(.*)<\/script>/.exec(addPlausible('<head></head>', origin))![1]!;
    const hash = createHash('sha256').update(queue).digest('base64');
    expect(directive('script-src')).toBe(`script-src 'self' 'wasm-unsafe-eval' ${origin} 'sha256-${hash}'`);
    expect(directive('connect-src')).toBe(`connect-src 'self' ${origin}`);
    expect(directive('default-src')).toBe("default-src 'self'");
    expect(policy).not.toContain('unsafe-inline');
  });
});
