import { bn, NativeAssetId, ZeroBytes32 } from 'fuels';

import type { ExchangeContractAbi, RegistryContractAbi } from '../../contracts';

const { TOKEN_AMOUNT, ETH_AMOUNT } = process.env;

export async function registerPool(
  registryContract: RegistryContractAbi,
  exchangeContract: ExchangeContractAbi,
  overrides: any
) {
  console.log('Registering pool');

  const root = await registryContract.functions.exchange_contract_root().get();
  if (root.value == ZeroBytes32) {
    console.log('Initializing registry');
    await registryContract.functions.initialize(exchangeContract.id.toB256())
      .txParams(overrides)
      .addContracts([exchangeContract.id])
      .call();
  } else {
    console.log('Registry already initialized');
  }

  const isRegistered = await registryContract.functions.is_pool(exchangeContract.id.toB256()).get();
  if (isRegistered.value) {
    console.log(`Exchange ${exchangeContract.id.toB256()} already registered`);
    return;
  }

  console.log(`Registering exchange ${exchangeContract.id.toB256()}`);
  await registryContract
    .functions.add_exchange_contract(exchangeContract.id.toB256())
    .txParams(overrides)
    .addContracts([exchangeContract.id])
    .call();
}
