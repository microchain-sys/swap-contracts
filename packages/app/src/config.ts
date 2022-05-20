import { parseUnits } from 'ethers/lib/utils';

export const FUEL_PROVIDER_URL =
  import.meta.env.VITE_FUEL_PROVIDER_URL || 'https://node.swayswap.io/graphql';

export const FUEL_FAUCET_URL =
  import.meta.env.VITE_FUEL_FAUCET_URL || 'https://faucet-fuel-core.swayswap.io/dispense';

export const CONTRACT_ID = import.meta.env.VITE_CONTRACT_ID!;
export const TOKEN_ID = import.meta.env.VITE_TOKEN_ID!;
export const DECIMAL_UNITS = 3;
export const FAUCET_AMOUNT = parseUnits('0.5', DECIMAL_UNITS).toBigInt();
export const MINT_AMOUNT = parseUnits('2000', DECIMAL_UNITS).toBigInt();
export const ONE_ASSET = parseUnits('1', DECIMAL_UNITS).toBigInt();
export const RECAPTCHA_SITE_KEY = import.meta.env.VITE_RECAPTCHA_SITE_KEY!;
export const ENABLE_FAUCET_API = import.meta.env.VITE_ENABLE_FAUCET_API === 'true';

// Max value supported
// eslint-disable-next-line @typescript-eslint/no-loss-of-precision
export const MAX_U64_VALUE = 0xffff_ffff_ffff_ffff;