import { bn, Wallet } from 'fuels';

import '../../load.envs';
import './loadDockerEnv';
import { ExchangeContractAbi__factory, RegistryContractAbi, RegistryContractAbi__factory, RouterContractAbi__factory, TokenContractAbi__factory } from '../../src/types/contracts';

import { initializePool } from './initializePool';
import { initializeTokenContract } from './initializeTokenContract';
import { registerPool } from './registerPool';

const { WALLET_SECRET, PROVIDER_URL, GAS_PRICE, VITE_CONTRACT_ID, VITE_TOKEN_ID, ROUTER_CONTRACT_ID, REGISTRY_CONTRACT_ID } =
  process.env;

if (!WALLET_SECRET) {
  process.stdout.write('WALLET_SECRET is not detected!\n');
  process.exit(1);
}

async function main() {
  const wallet = new Wallet(WALLET_SECRET!, PROVIDER_URL);

  if (!ROUTER_CONTRACT_ID || !VITE_CONTRACT_ID || !VITE_TOKEN_ID || !REGISTRY_CONTRACT_ID) {
    console.error('Contract addresses missing');
    console.error({ ROUTER_CONTRACT_ID, VITE_CONTRACT_ID, VITE_TOKEN_ID, REGISTRY_CONTRACT_ID });
    return
  }

  const routerContract = RouterContractAbi__factory.connect(ROUTER_CONTRACT_ID!, wallet);
  const registryContract = RegistryContractAbi__factory.connect(REGISTRY_CONTRACT_ID, wallet);
  const exchangeContract = ExchangeContractAbi__factory.connect(VITE_CONTRACT_ID!, wallet);
  const tokenContract = TokenContractAbi__factory.connect(VITE_TOKEN_ID!, wallet);
  const overrides = {
    gasPrice: bn(GAS_PRICE || 0),
  };

  await initializeTokenContract(tokenContract, overrides);
  await initializePool(routerContract, tokenContract, exchangeContract, overrides);
  await registerPool(registryContract, exchangeContract, overrides);
}

main();
