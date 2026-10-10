const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

const currentMajor = parseInt(process.versions.node.split('.')[0], 10);
let npxBin = process.platform === 'win32' ? 'npx.cmd' : 'npx';

if (currentMajor < 20) {
  const home = os.homedir();
  const nvmDir = path.join(home, '.nvm', 'versions', 'node');
  if (fs.existsSync(nvmDir)) {
    const versions = fs.readdirSync(nvmDir).filter(v => v.startsWith('v20') || v.startsWith('v22') || v.startsWith('v24')).sort();
    if (versions.length > 0) {
      const best = versions[versions.length - 1];
      const binDir = path.join(nvmDir, best, 'bin');
      const candidate = path.join(binDir, 'npx');
      if (fs.existsSync(candidate)) {
        npxBin = candidate;
        process.env.PATH = `${binDir}${path.delimiter}${process.env.PATH}`;
      }
    }
  }
}

const args = process.argv.slice(2);
const res = spawnSync(npxBin, args, { stdio: 'inherit', shell: true });
process.exit(res.status !== null ? res.status : 0);
