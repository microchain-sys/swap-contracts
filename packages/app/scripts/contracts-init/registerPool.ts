import { bn, NativeAssetId } from 'fuels';

import type { ExchangeContractAbi, RegistryContractAbi, RouterContractAbi, TokenContractAbi } from '../../src/types/contracts';

const { TOKEN_AMOUNT, ETH_AMOUNT } = process.env;

export async function registerPool(
  registryContract: RegistryContractAbi,
  exchangeContract: ExchangeContractAbi,
  overrides: any
) {
  console.log('Registering pool');

  await registryContract
    .functions.add_exchange_contract(exchangeContract.id.toB256())
    .txParams({
      ...overrides,
      variableOutputs: 2,
      gasLimit: 100_000_000,
    })
    .addContracts([
      exchangeContract.id,
    ])
    .call();
}
