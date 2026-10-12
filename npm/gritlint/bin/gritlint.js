#!/usr/bin/env node
'use strict';

const { spawnSync } = require('node:child_process');
const path = require('node:path');

const run = () => {
  const { platform, arch } = process;
  if (platform !== 'linux' || (arch !== 'x64' && arch !== 'arm64')) {
    process.stderr.write(
      `gritlint: no binary for ${platform}-${arch}; this package ships linux-x64 and linux-arm64\n`,
    );
    return 2;
  }

  const binary = path.join(__dirname, `gritlint-linux-${arch}`);
  const child = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });
  if (child.error) {
    process.stderr.write(`gritlint: cannot run ${binary}: ${child.error.message}\n`);
    return 2;
  }

  if (child.signal) {
    process.kill(process.pid, child.signal);
  }
  return child.status === null ? 2 : child.status;
};

process.exitCode = run();
