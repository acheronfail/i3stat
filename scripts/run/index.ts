#!/usr/bin/env node

import { execa } from 'execa';
import { parse, HTMLElement } from 'node-html-parser';
import chalk from 'chalk';
import stripAnsi from 'strip-ansi';

process.chdir('../..');

// spawn, pipe stdout/err and write initial stdin
const child = execa('./target/debug/staturs');
child.stderr.pipe(process.stderr);
child.stdin.write('[\n');

// custom stdout listening for pango markup
let isOutputPaused = false;
child.stdout.setEncoding('utf8');
child.stdout.on('data', (input: string) => {
  const lines = input.split('\n');
  for (let i = 0; i < lines.length; ++i) {
    let line = lines[i];
    if (line.endsWith('],')) {
      line = formatLine(line);
    }

    if (isOutputPaused) continue;

    const pad = ' '.repeat(process.stdout.columns - stripAnsi(line).length);
    process.stdout.cursorTo(0, process.stdout.rows - 1);
    process.stdout.clearLine(0);
    process.stdout.write(pad + line);
    if (i < lines.length - 1) {
      process.stdout.write('\n');
    }
  }

  drawInterface();
});

// listen for commands on stdin, and send JSON to child
process.stdin.setEncoding('utf8');
process.stdin.setRawMode(true);
process.stdin.resume();

const CTRL_C = '\x03';
const BACKSP = '\x7f';
const RETURN = '\r';
const UP_ARR = '\x1B[A';
const DN_ARR = '\x1B[B';
let prev_commands: string[] = [];
let prev_command_idx = 0;
let displayShort = false;
let _input = '';

process.stdin.on('data', (char: string) => {
  // NOTE: console.log(Buffer.from(char));
  if (char === CTRL_C) {
    process.exit(0);
  } else if (char === BACKSP) {
    _input = _input.slice(0, -1);
  } else if (char == RETURN) {
    handleInput(_input.trim());
    _input = '';
  } else if (char === UP_ARR) {
    if (prev_commands.length) {
      prev_command_idx = prev_command_idx > 0 ? prev_command_idx - 1 : 0;
      _input = prev_commands[prev_command_idx] || '';
    }
  } else if (char === DN_ARR) {
    if (prev_commands.length) {
      prev_command_idx = prev_command_idx == prev_commands.length ? prev_commands.length : prev_command_idx + 1;
      _input = prev_commands[prev_command_idx] || '';
    }
  } else if (char.startsWith('\x1B[')) {
    // just ignore all escape characters for now...
    // should use this to implement a cursor...
    _input = '';
  } else {
    _input += char;
  }

  drawInterface();
});

function drawInterface() {
  // draw instance info in line
  const info = instances.map((i) => `${i.id || '?'}: ${i.name}`).join(', ');
  const pad = ' '.repeat(process.stdout.columns - info.length);
  process.stdout.cursorTo(0, process.stdout.rows);
  process.stdout.write(pad + chalk.grey.italic(info));

  // draw input line
  process.stdout.cursorTo(0, process.stdout.rows);
  process.stdout.write(`${displayShort ? 'short' : ' full'}> ${_input}`);
}

function displayHelp() {
  process.stderr.write(chalk.grey.italic`
Usage:
  [c]lick <instance> <button>       e.g.: "click 0 3"
  [l]ist                            lists bar items with instance ids
  [p]ause                           toggles pausing output
  [s]hort                           toggles between full and short text
  [r]epeat                          repeats last command
  [h]elp or ?                       show this text
  [q]uit                            exits
`);
}

function handleInput(input: string) {
  // help
  if (input.startsWith('?') || input == 'h' || input == 'help') return displayHelp();

  // repeat
  if (input == 'r' || input == 'repeat') {
    if (!prev_commands.length) {
      return displayHelp();
    }

    input = prev_commands[prev_commands.length - 1];
  }

  // short
  else if (input == 's' || input == 'short') {
    displayShort = !displayShort;
  }

  // pause
  else if (input == 'p' || input == 'pause') {
    isOutputPaused = !isOutputPaused;
  }

  // quit
  else if (input == 'q' || input == 'quit') {
    process.exit();
  }

  // list
  else if (input == 'l' || input == 'list') {
    const c = chalk.gray.italic;
    process.stdout.write(c('\n'));
    for (const { name, id } of instances) {
      process.stdout.write(c(`${id || '?'}: ${name}\n`));
    }
    process.stdout.write(c('\n'));
  }

  // click
  else if (input.startsWith('c')) {
    const match = /c(?:lick)?\s+(\d)\s+(\d)(?:\s+(s))?/.exec(input);
    if (!match) return displayHelp();

    const [, block, btn, shift] = match;
    const click = {
      name: null,
      instance: block,
      button: parseInt(btn),
      modifiers: shift ? ['Shift'] : [],
      x: 0,
      y: 0,
      relative_x: 0,
      relative_y: 0,
      output_x: 0,
      output_y: 0,
      width: 10,
      height: 10,
    };

    child.stdin.write(JSON.stringify(click) + '\n');
  } else {
    return displayHelp();
  }

  if (input != prev_commands[prev_commands.length - 1]) {
    prev_command_idx = prev_commands.push(input);
  }
}

let instances: { name: string; id: string }[] = [];
function formatLine(line: string) {
  const items: Record<string, any>[] = JSON.parse(line.slice(0, -1));
  instances = items.map((i) => ({ name: i.name, id: i.instance }));
  const getText = (item: Record<string, any>) => {
    const text = item[displayShort ? 'short_text' : 'full_text'];
    return text || item.full_text;
  };

  let result: string[] = [];
  for (const item of items) {
    const text = getText(item);
    if (!text) {
      continue;
    }

    if (item.markup !== 'pango') {
      result.push(text);
      continue;
    }

    const root = parse(text);
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
