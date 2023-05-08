#!/usr/bin/env node

import * as cp from 'child_process';
import { parse, HTMLElement } from 'node-html-parser';
import chalk from 'chalk';
import stripAnsi from 'strip-ansi';

process.chdir('../..');

function displayHelp() {
  process.stderr.write(chalk.grey.italic`Usage:
  click <instance> <button>       e.g.: "click 0 3"
  repeat                          repeats last command
`);
}

// spawn, pipe stdout/err and write initial stdin
const child = cp.spawn('./target/debug/staturs');
child.stderr.pipe(process.stderr);
child.stdin.write('[\n');

// custom stdout listening for pango markup
child.stdout.setEncoding('utf8');
child.stdout.on('data', (input) => {
  const lines = input.split('\n');
  for (let i = 0; i < lines.length; ++i) {
    let line = lines[i];
    if (line.endsWith('],')) {
      line = formatLine(line);
    }

    const pad = ' '.repeat(process.stdout.columns - stripAnsi(line).length);
    process.stdout.write('\r' + pad + line);
    if (i < lines.length - 1) {
      process.stdout.write('\n');
    }
  }
});

// listen for commands on stdin, and send JSON to child
// TODO: have this on a separate line, so we can see what we're typing (like a repl prompt)
process.stdin.setEncoding('utf8');
let last_cmd: string | null = null;
process.stdin.on('data', (input: string) => {
  if (input.startsWith('?') || input.startsWith('h')) return displayHelp();

  if (input.startsWith('r')) {
    if (!last_cmd) {
      return displayHelp();
    }

    input = last_cmd;
  }

  const match = /c(?:lick)?\s+(\d)\s+(\d)\s+(s)?/.exec(input);
  if (!match) return displayHelp();

  const [, block, btn, shift] = match;
  const click = {
    name: null,
    instance: block,
    button: parseInt(btn),
    modifiers: shift ? ['Shift'] : [],
    x: 11,
    y: 12,
    relative_x: 15,
    relative_y: 16,
    output_x: 9,
    output_y: 8,
    width: 13,
    height: 14,
  };

  child.stdin.write(JSON.stringify(click) + '\n');
  last_cmd = input;
});

function formatLine(line: string) {
  const items: any[] = JSON.parse(line.slice(0, -1));

  let result: any[] = [];
  for (const item of items) {
    if (!item.full_text) {
      continue;
    }

    if (item.markup !== 'pango') {
      result.push(item.full_text);
      continue;
    }

    const root = parse(item.full_text);
    for (const node of root.childNodes) {
      if (node instanceof HTMLElement) {
        const { foreground, background } = node.attributes;
        let c = chalk;
        if (foreground) c = c.hex(foreground);
        if (background) c = c.bgHex(background);
        node.innerHTML = c(node.textContent);
      }
    }

    // TODO: border?
    let c = chalk;
    if (item.color) c = c.hex(item.color);
    if (item.background) c = c.bgHex(item.background);
    result.push(c(root.textContent));
  }

  return result.join(chalk.gray('|'));
}