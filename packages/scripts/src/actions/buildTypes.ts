import { runTypegen } from '@fuel-ts/abi-typegen/runTypegen';
import type { Config } from 'src/types';

// Generate types using fuels/abi-typegen
export async function buildTypes(config: Config) {
  const cwd = process.cwd();
  await runTypegen({
    cwd,
    input: config.types.artifacts,
    output: config.types.output,
  });
}
