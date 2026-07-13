#!/usr/bin/env python3
"""Build script: minify CSS/JS and inline into a single bundled HTML file.

Usage: python3 build_dashboard.py
Reads:  static/index.html, static/app.css, static/app.js
Writes: static/bundled.html
"""

import re
from pathlib import Path


def minify_css(text: str) -> str:
    """Minify CSS: strip comments, collapse whitespace."""
    text = re.sub(r'/\*.*?\*/', '', text, flags=re.DOTALL)
    text = text.replace('\n', ' ').replace('\r', '')
    text = re.sub(r'\s+', ' ', text)
    text = re.sub(r'\s*([{}:;,+>~])\s*', r'\1', text)
    text = re.sub(r'\s*({|;)', r'\1', text)
    return text.strip()


def minify_js(text: str) -> str:
    """Minify JS: strip comments, collapse newlines.

    SAFE for template literals (backticks) — passes them through unchanged.
    """
    result = []
    i = 0
    length = len(text)

    while i < length:
        c = text[i]

        # Template literals (backticks) — pass through entirely unchanged
        if c == '`':
            result.append(c)
            i += 1
            depth = 0
            while i < length:
                ch = text[i]
                if ch == '\\' and i + 1 < length:
                    result.append(text[i:i+2])
                    i += 2
                    continue
                if ch == '`' and depth == 0:
                    result.append('`')
                    i += 1
                    break
                if ch == '$' and i + 1 < length and text[i+1] == '{':
                    depth += 1
                if ch == '}' and depth > 0:
                    depth -= 1
                result.append(ch)
                i += 1
            continue

        # String literals (single/double quotes) — pass through unchanged
        if c in ('"', "'"):
            quote = c
            result.append(c)
            i += 1
            while i < length and text[i] != quote:
                if text[i] == '\\' and i + 1 < length:
                    result.append(text[i:i+2])
                    i += 2
                    continue
                result.append(text[i])
                i += 1
            if i < length:
                result.append(text[i])
                i += 1
            continue

        # Block comments
        if c == '/' and i + 1 < length and text[i+1] == '*':
            end = text.find('*/', i + 2)
            i = end + 2 if end != -1 else length
            continue

        # Single-line comments
        if c == '/' and i + 1 < length and text[i+1] == '/':
            end = text.find('\n', i)
            i = end + 1 if end != -1 else length
            continue

        # Collapse newlines/carriage returns into single space
        if c in ('\n', '\r'):
            if result and result[-1] != ' ':
                result.append(' ')
            i += 1
            continue

        result.append(c)
        i += 1

    return ''.join(result).strip()


def bundle():
    """Read source files, minify, inline, and write bundled.html."""
    static_dir = Path('static')

    html = (static_dir / 'index.html').read_text(encoding='utf-8')
    css = minify_css((static_dir / 'app.css').read_text(encoding='utf-8'))
    js = minify_js((static_dir / 'app.js').read_text(encoding='utf-8'))

    html = html.replace(
        '<link rel="stylesheet" href="/static/app.css">',
        f'<style>{css}</style>'
    )
    html = html.replace(
        '<script src="/static/app.js" defer></script>',
        f'<script>{js}</script>'
    )

    output = static_dir / 'bundled.html'
    output.write_text(html, encoding='utf-8')

    print(f'CSS: {len(css):,} bytes')
    print(f'JS:  {len(js):,} bytes')
    print(f'HTML: {len(html):,} bytes (bundled)')


if __name__ == '__main__':
    bundle()