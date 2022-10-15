import { runTypeChain, glob } from 'fuelchain';
import type { Config } from 'src/types';
import { promises as fs } from 'fs';

// Generate types using typechain
// and typechain-target-fuels modules
export async function buildTypes(config: Config) {
  const cwd = process.cwd();
  // find all files matching the glob
  const allFiles = glob(cwd, [config.types.artifacts]);

  // Hack until https://github.com/FuelLabs/fuels-ts/issues/521 is resolved
  const searchStr = '"name": "get_tokens",\n      "output": {\n        "name": ""';
  const replaceStr = '"name": "get_tokens",\n      "output": {\n        "name": "tokens"';
  for (const filePath of allFiles) {
    const code = await fs.readFile(filePath, { encoding: 'utf8' });
    if (code.indexOf(searchStr) !== -1) {
      fs.writeFile(filePath, code.replace(searchStr, replaceStr));
    }
  }

  await runTypeChain({
    cwd,
    filesToProcess: allFiles,
    allFiles,
    outDir: config.types.output,
    target: 'fuels',
  });
}
