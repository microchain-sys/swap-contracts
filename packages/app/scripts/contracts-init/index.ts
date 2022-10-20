import { bn, Wallet } from 'fuels';

import '../../load.envs';
import './loadDockerEnv';
import { ExchangeContractAbi__factory, RegistryContractAbi, RegistryContractAbi__factory, RouterContractAbi__factory, TokenContractAbi__factory } from '../../src/types/contracts';

import { initializePool } from './initializePool';
import { initializeTokenContract } from './initializeTokenContract';
import { registerPool } from './registerPool';

const {
  WALLET_SECRET,
  PROVIDER_URL,
  GAS_PRICE,
  VITE_EXCHANGE_1_ID,
  VITE_EXCHANGE_2_ID,
  VITE_TOKEN_1_ID,
  VITE_TOKEN_2_ID,
  ROUTER_CONTRACT_ID,
  REGISTRY_CONTRACT_ID
} = process.env;

if (!WALLET_SECRET) {
  process.stdout.write('WALLET_SECRET is not detected!\n');
  process.exit(1);
}

async function main() {
  const wallet = new Wallet(WALLET_SECRET!, PROVIDER_URL);

  if (!ROUTER_CONTRACT_ID || !VITE_EXCHANGE_1_ID || !VITE_EXCHANGE_2_ID || !VITE_TOKEN_1_ID || !VITE_TOKEN_2_ID || !REGISTRY_CONTRACT_ID) {
    console.error('Contract addresses missing');
    console.error({ ROUTER_CONTRACT_ID, VITE_EXCHANGE_1_ID, VITE_EXCHANGE_2_ID, VITE_TOKEN_1_ID, VITE_TOKEN_2_ID, REGISTRY_CONTRACT_ID });
    return
  }

  const routerContract = RouterContractAbi__factory.connect(ROUTER_CONTRACT_ID!, wallet);
  const registryContract = RegistryContractAbi__factory.connect(REGISTRY_CONTRACT_ID, wallet);
  const exchange1Contract = ExchangeContractAbi__factory.connect(VITE_EXCHANGE_1_ID!, wallet);
  const exchange2Contract = ExchangeContractAbi__factory.connect(VITE_EXCHANGE_2_ID!, wallet);
  const token1Contract = TokenContractAbi__factory.connect(VITE_TOKEN_1_ID!, wallet);
  const token2Contract = TokenContractAbi__factory.connect(VITE_TOKEN_2_ID!, wallet);
  const overrides = {
    gasPrice: bn(GAS_PRICE || 0),
  };

  await initializeTokenContract(token1Contract, overrides);
  await initializeTokenContract(token2Contract, overrides);
  await initializePool(routerContract, token1Contract, exchange1Contract, overrides);
  await initializePool(routerContract, token2Contract, exchange2Contract, overrides);
  await registerPool(registryContract, exchange1Contract, overrides);
  await registerPool(registryContract, exchange2Contract, overrides);
}

main();
