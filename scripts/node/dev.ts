#!/usr/bin/env node
/// <reference path="./node_modules/@types/node/index.d.ts" />

import { execa, $ } from 'execa';
import { parse, HTMLElement } from 'node-html-parser';
import chalk from 'chalk';
import { createConnection } from 'net';
import stripAnsi from 'strip-ansi';
import cssColors from 'css-color-names';
import { Bin, subscript, superscript } from './consts';

enum Display {
  Full = 'full',
  Short = 'short',
  Json = 'json',
}

const CTRL_C = '\x03';
const CTRL_D = '\x04';
const BACKSP = '\x7f';
const RETURN = '\r';
const UP_ARR = '\x1B[A';
const DN_ARR = '\x1B[B';
const SOCKET_PATH = '/tmp/istat-socket.dev';

let prev_commands: string[] = [];
let prev_command_idx = 0;
let isOutputPaused = false;
let display: Display = Display.Full;
let filter: string[] = [];
let _input = '';

// spawn, pipe stdout/err and write initial stdin
process.chdir('../..');
const { sigrtmin, sigrtmax } = JSON.parse((await $`${Bin.iStatSignals}`).stdout);
const child = execa(Bin.iStat, ['--config=./sample_config.toml', `--socket=${SOCKET_PATH}`]);
if (!child.stdin) throw new Error("Child's STDIN was not setup correctly!");
if (!child.stdout) throw new Error("Child's STDOUT was not setup correctly!");
if (!child.stderr) throw new Error("Child's STDERR was not setup correctly!");

child.stdin.write('[\n');

// exit if the child exits unexpectedly
child.on('exit', (code: number, signal: string) => {
  process.stdout.write(chalk.red(`Exited: ${code || signal}\n`));
  process.exit(0);
});

// custom stdout listening for pango markup
child.stdout.setEncoding('utf8');
child.stdout.on('data', (output: string) => {
  const lines = output.split('\n');
  for (let i = 0; i < lines.length; ++i) {
    let line = lines[i];
    if (line.endsWith('],')) {
      line = formatLine(line);
    }

    if (isOutputPaused) continue;

    const pad = ' '.repeat(Math.max(0, process.stdout.columns - stripAnsi(line).length));
    process.stdout.cursorTo(0, process.stdout.rows - 1);
    process.stdout.clearLine(0);
    process.stdout.write(pad + line);
    if (i < lines.length - 1) {
      process.stdout.write('\n');
    }
  }

  drawInterface();
});

// catch stderr and display it nicely
child.stderr.setEncoding('utf8');
child.stderr.on('data', (output: string) => {
  process.stdout.cursorTo(0, process.stdout.rows - 1);
  process.stdout.clearLine(0);
  process.stdout.write(chalk.hex('#ffa500')(output));
});

// listen for commands on stdin, and send JSON to child
process.stdin.setEncoding('utf8');
process.stdin.setRawMode(true);
process.stdin.resume();
process.stdin.on('data', (char: string) => {
  // NOTE: console.log(Buffer.from(char));
  if (char === CTRL_C || char === CTRL_D) {
    exit(0);
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

for (const cmd of process.argv.slice(2)) {
  handleInput(cmd);
}

function drawInterface() {
  // draw instance info in line
  const info = instances.map((i) => `${i.id || '?'}: ${i.name}`).join(', ');
  const padCount = process.stdout.columns - info.length;
  const pad = padCount > 0 ? ' '.repeat(padCount) : '';
  process.stdout.cursorTo(0, process.stdout.rows);
  process.stdout.write(pad + chalk.grey.italic(info));

  // draw input line
  process.stdout.cursorTo(0, process.stdout.rows);
  const d = isOutputPaused ? 'paused' : display;
  const f = filter.length ? chalk.gray(`(${filter.join(',')})`) : '';
  process.stdout.write(`${d}${f}> ${_input}`);
}

function displayHelp() {
  process.stderr.write(chalk.grey.italic`
Usage:
  [c]lick <instance> <button>       e.g.: "click 0 3"
  [f]ilter [1,2,3,...]              only show particular items, leave empty to reset
  [l]ist                            lists bar items with instance ids
  [d]isplay [full|short|json]       change display, or empty to rotate between them
  [p]ause                           toggles pausing output
  [r]epeat                          repeats last command
  [R]efresh                         refreshes all items
  [s]ignal <instance> <signal>      sends a realtime signal
  [h]elp or ?                       show this text
  [q]uit                            exits
`);
}

function handleInput(input: string) {
  const c = chalk.gray.italic;

  // help
  if (input.startsWith('?') || input == 'h' || input == 'help') return displayHelp();

  // repeat
  if (input == 'r' || input == 'repeat') {
    if (!prev_commands.length) {
      return displayHelp();
    }

    input = prev_commands[prev_commands.length - 1];
  }

  if (input.startsWith('R')) {
    const match = /R(?:efresh)?/.exec(input);
    if (!match) return displayHelp();

    const socket = createConnection(SOCKET_PATH);
    socket.once('connect', () => {
      const message = Buffer.from('"refresh_all"');
      const header = Buffer.alloc(8);
      header.writeBigUInt64LE(BigInt(message.length));
      const payload = Buffer.concat([header, message]);
      socket.write(payload);
      socket.on('data', (data) => {
        // first 8 bytes are the header
        const message = data.subarray(8);
        process.stdout.clearLine(0);
        process.stdout.write(c.green(`Refreshed all items. IPC response: ${message.toString()}\n`));
      });
    });
  }

  // short
  else if (input.startsWith('d')) {
    const match = /d(?:isplay)?(?:\s+(full|short|json))?/.exec(input);
    if (!match) return displayHelp();

    const [, value] = match;
    if (value) {
      display = value as Display;
    } else {
      switch (display) {
        case Display.Full:
          display = Display.Short;
          break;
        case Display.Short:
          display = Display.Json;
          break;
        case Display.Json:
          display = Display.Full;
          break;
      }
    }
  }

  // json
  else if (input == 'j' || input == 'short') {
    display = display == Display.Short ? Display.Full : Display.Short;
  }

  // filter
  else if (input.startsWith('f')) {
    const match = /f(?:ilter)?(?:\s+((?:\d,?)+))?/.exec(input);
    if (!match) return displayHelp();

    const [, ids] = match;
    if (!ids) filter.length = 0;
    else filter = ids.split(',');
  }

  // pause
  else if (input == 'p' || input == 'pause') {
    isOutputPaused = !isOutputPaused;
  }

  // quit
  else if (input == 'q' || input == 'quit') {
    exit(0);
  }

  // signal
  else if (input.startsWith('s')) {
    const match = /s(?:ignal)?\s+(\d+)/.exec(input);
    if (!match) return displayHelp();

    const [, signalStr] = match;
    process.stdout.write(c(`\nSending signal: SIGRTMIN+${signalStr}\n`));

    const signal = sigrtmin + parseInt(signalStr);
    if (signal < 0 || signal > sigrtmax) {
      process.stdout.write(c`\n`);
      process.stdout.write(c.red(`Invalid signal: ${signalStr}\n`));
      process.stdout.write(c.red(`Valid realtime signals range: 0..${sigrtmax - sigrtmin}`));
      process.stdout.write(c`\n`);
      return;
    }

    // not exactly a public API, but I'm glad it exists since `child.kill(signal)` throws with `ERR_UNKNOWN_SIGNAL`
    // for realtime signals, see:
    // https://github.com/nodejs/node/blob/0b3fcfcf351fba9f29234976eeec4afb09ae2cc0/src/node_process_methods.cc#L145
    // https://github.com/nodejs/node/blob/0b3fcfcf351fba9f29234976eeec4afb09ae2cc0/src/node_process_methods.cc#L597
    (process as any)._kill((child as any)._handle.pid, signal);
  }

  // list
  else if (input == 'l' || input == 'list') {
    process.stdout.write(c('\n'));
    for (const { name, id } of instances) {
      process.stdout.write(c(`${id || '?'}: ${name}\n`));
    }
    process.stdout.write(c('\n'));
  }

  // click
  else if (input.startsWith('c')) {
    const match = /c(?:lick)?\s+(\d+)\s+(\d)(?:\s+(s))?/.exec(input);
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

    child.stdin!.write(JSON.stringify(click) + '\n');
  } else {
    return displayHelp();
  }

  if (input != prev_commands[prev_commands.length - 1]) {
    prev_command_idx = prev_commands.push(input);
  }
}

let instances: { name: string; id: string }[] = [];
const sep = chalk.gray('|');
function formatLine(line: string) {
  const items: Record<string, any>[] = JSON.parse(line.slice(0, -1));
  // extract bar item instances from JSON output
  instances = items
    // filter out powerline separator items
    .filter((i) => !i._powerline_sep)
    .map((i) => ({ name: i.name, id: i.instance }));

  const getText = (item: Record<string, any>) => item[`${display}_text`] || item.full_text;

  let result: string[] = [];
  for (const [i, item] of items.entries()) {
    if (filter.length && !filter.includes(item.instance)) {
      continue;
    }

    if (display == Display.Json) {
      result.push(JSON.stringify(item));
      continue;
    }

    const text = getText(item);
    if (!text) {
      continue;
    }

    const hasSeparator = !('separator' in item) || item.separator;
    if (item.markup !== 'pango') {
      result.push(c(item.color, item.background, item.urgent)(text));
      if (hasSeparator) result.push(sep);
      continue;
    }

    const root = parse(text);
    for (const node of root.childNodes) {
      if (node instanceof HTMLElement) {
        const { foreground, background } = node.attributes;
        // quick and dirty replacement of <sup> <sub> with some unicode equivalents
        for (const child of node.childNodes) {
          if (child instanceof HTMLElement) {
            if (child.tagName.toLowerCase() == 'sub') {
              child.innerHTML = replaceWithMap(child.textContent, subscript);
            }
            if (child.tagName.toLowerCase() == 'sup') {
              child.innerHTML = replaceWithMap(child.textContent, superscript);
            }
            child.innerHTML.replace(/\//g, '⁄');
          }
        }

        // color spans
        node.innerHTML = c(foreground, background, item.urgent)(node.textContent);
      }
    }

    // NOTE: AFAICT there's no way to draw a border in the terminal, so we can't display that here
    result.push(c(item.color, item.background, item.urgent)(root.textContent));

    if (hasSeparator && i < items.length - 1) result.push(sep);
  }

  return result.join('');
}

function replaceWithMap(s: string, map: Record<string, string>) {
  return s
    .split('')
    .map((ch) => map[ch] ?? ch)
    .join('');
}

function c(fg: string | undefined, bg: string | undefined, urgent: boolean) {
  let fmt = chalk;
  if (urgent) {
    fmt = fmt.bgRedBright;
    fmt = fg ? fmt.hex(fg.startsWith('#') ? fg : cssColors[fg]) : fmt.black;
    return fmt;
  }

  if (fg) fmt = fmt.hex(fg.startsWith('#') ? fg : cssColors[fg]);
  if (bg) fmt = fmt.bgHex(bg.startsWith('#') ? bg : cssColors[bg]);
  return fmt;
}

function exit(code: number) {
  const doExit = () => {
    process.stdout.cursorTo(0, process.stdout.rows);
    process.stdout.clearLine(0);
    process.exit(code);
  };

  child.removeAllListeners('exit').on('exit', doExit);
  child.kill('SIGTERM');

  setTimeout(doExit, 5_000);
}
